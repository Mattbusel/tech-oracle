//! THE BLOODLINE: the oracle is not one strategy, it is a breeding population.
//!
//! Each organism carries strategy genes (how bold a line it sets, how much it
//! chases longshots). Every day, all of them shadow-bet the entire resolved
//! record with their own instincts; the richest survive, the broke ones die, the
//! survivors mate and mutate, and the fittest organism's genes drive the live
//! oracle. A real genetic algorithm, speciating in public. Persisted to
//! data/bloodline.json. Deterministic per date.

use crate::model::Prediction;
use serde::{Deserialize, Serialize};

const PATH: &str = "data/bloodline.json";
const TARGET: usize = 12; // living population size
const SURVIVORS: usize = 5; // how many live through each cull (kill ~7, breed ~7 a day: fast churn)
const KEEP_DEAD: usize = 40; // graveyard depth
const JUDGE_MIN: usize = 5; // settled calls needed before any culling

const NAMES: &[&str] = &[
    "RIBBON", "COPPER", "VECTOR", "CIPHER", "EMBER", "QUARTZ", "PISTON", "LANTERN", "STYLUS",
    "CARBON", "RELAY", "DELTA", "OXIDE", "PRISM", "ANVIL", "COBALT", "HALIDE", "INDIGO", "KELVIN",
    "LUMEN", "MAGNET", "ONYX", "QUASAR", "ROTOR", "SOLDER", "ULTRA", "VOLT", "WAFER", "XENON",
    "ZINC", "BRASS", "CEDAR", "DRIFT", "FLINT", "GLASS", "IRON", "JADE", "NOVA", "PEARL", "SLATE",
];

// Six strategy genes give a real behavior space: not just how bold the line is,
// but how selective, how it presses a hot hand, and whether it tails or fades
// the oracle entirely. A whole personality, heritable and mutable.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Genes {
    pub aggr: f64,   // -0.12..0.12  line aggressiveness (shifts its confidence)
    pub risk: f64,   // 0..1         stake variance / longshot appetite
    pub conf: f64,   // -0.08..0.08  baseline confidence bias
    #[serde(default)]
    pub select: f64, // 0..1         selectivity: how high a bar before it bets
    #[serde(default)]
    pub press: f64,  // 0..1         conviction: how hard it ramps stake on a hot streak
    #[serde(default)]
    pub fade: f64,   // 0..1         >0.5 = contrarian, it bets AGAINST the oracle
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Organism {
    pub id: u64,
    pub name: String,
    pub born: String,
    pub parents: Vec<u64>,
    pub genes: Genes,
    pub fitness: f64, // current shadow bankroll over the whole record
    pub age: i64,
    pub alive: bool,
    pub died: String,
    // The stat line: what makes a run impressive and a card worth holding.
    #[serde(default)]
    pub best: f64, // career-high bankroll ever reached
    #[serde(default)]
    pub bets: i64,
    #[serde(default)]
    pub wins: i64,
    #[serde(default)]
    pub losses: i64,
    #[serde(default)]
    pub win_rate: i64, // %
    #[serde(default)]
    pub max_streak: i64, // longest winning run
    #[serde(default)]
    pub biggest: i64, // biggest single win
    #[serde(default)]
    pub big_bet: i64, // largest single stake it shoved
    #[serde(default)]
    pub roi: i64, // % return on the 1000-chip stake
}

#[derive(Serialize, Deserialize, Default)]
pub struct Bloodline {
    pub next_id: u64,
    pub gen: i64,
    pub population: Vec<Organism>,
    #[serde(default)]
    pub last_evolved: String, // date of the last aging/cull/breed (idempotent per day)
    #[serde(default)]
    pub hall_of_fame: Vec<Organism>, // the all-time greats, never pruned
}

/// The full outcome of one organism shadow-betting the record.
struct SimStats {
    bank: f64,
    bets: i64,
    wins: i64,
    losses: i64,
    max_streak: i64,
    biggest: f64,  // biggest single win
    peak: f64,
    big_bet: f64,  // largest single stake it ever shoved
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + self.unit() * (b - a)
    }
}

fn ghash(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628257);
    }
    h
}

fn conf_of(p: &Prediction) -> f64 {
    if p.confidence > 0.0 { p.confidence } else { 0.65 }
}

/// One organism shadow-betting the record ONCE with its own genes. It bets a
/// FRACTION OF ITS BANKROLL (5%-100% by `risk`), pressed on a hot streak, so a
/// bold run compounds into a fortune or busts to zero. One bet per settled call
/// (shuffled per organism, so streaks and busts differ): a perfect run is
/// impossible over a real, diverse record, which keeps the bankrolls sane while
/// still swinging hard. `seed` makes it deterministic per organism.
fn simulate(g: &Genes, calls: &[(f64, bool)], seed: u64) -> SimStats {
    let n = calls.len();
    if n == 0 {
        return SimStats { bank: 1000.0, bets: 0, wins: 0, losses: 0, max_streak: 0, biggest: 0.0, peak: 1000.0, big_bet: 0.0 };
    }
    let mut rng = Rng(seed | 1);
    let mut bank = 1000.0_f64;
    let mut streak = 0i64;
    let (mut bets, mut wins, mut losses, mut max_streak) = (0i64, 0i64, 0i64, 0i64);
    let (mut biggest, mut peak, mut big_bet) = (0.0f64, 1000.0f64, 0.0f64);
    let bar = 0.34 + g.select * 0.30;
    let fading = g.fade > 0.5;
    let frac = 0.05 + g.risk * 0.95; // 5%..100%: no guardrail, free to shove it all
    // this organism's own run of luck through the record
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let j = (rng.next() as usize) % (i + 1);
        order.swap(i, j);
    }
    for &idx in &order {
        let (conf, hit) = calls[idx];
        let c = (conf + g.aggr + g.conf).clamp(0.34, 0.95);
        if c < bar {
            continue; // only the pickiest skip the lowest-confidence calls
        }
        let won = if fading { !hit } else { hit };
        let p = if fading { (1.0 - c).max(0.05) } else { c };
        let ramp = 1.0 + g.press * (streak.min(6) as f64) * 0.7;
        let stake = (bank * frac * ramp).min(bank).max(1.0);
        if stake > big_bet { big_bet = stake; }
        bets += 1;
        if won {
            let w = stake * ((1.0 / p) - 1.0);
            bank += w;
            wins += 1;
            streak += 1;
            if streak > max_streak { max_streak = streak; }
            if w > biggest { biggest = w; }
        } else {
            bank -= stake;
            losses += 1;
            streak = 0;
        }
        if bank > peak { peak = bank; }
        if bank < 1.0 {
            bank = 0.0;
            break; // busted out
        }
    }
    SimStats { bank, bets, wins, losses, max_streak, biggest, peak, big_bet }
}

fn random_genes(r: &mut Rng) -> Genes {
    // Wide-open ranges so the founding population is genuinely diverse.
    Genes {
        aggr: r.range(-0.20, 0.20),
        risk: r.range(0.0, 1.0),
        conf: r.range(-0.15, 0.15),
        select: r.range(0.0, 1.0),
        press: r.range(0.0, 1.0),
        fade: r.range(0.0, 1.0),
    }
}

fn crossover(a: &Genes, b: &Genes, r: &mut Rng) -> Genes {
    let pick = |x: f64, y: f64, r: &mut Rng| if r.unit() < 0.5 { x } else { y };
    let mut g = Genes {
        aggr: pick(a.aggr, b.aggr, r),
        risk: pick(a.risk, b.risk, r),
        conf: pick(a.conf, b.conf, r),
        select: pick(a.select, b.select, r),
        press: pick(a.press, b.press, r),
        fade: pick(a.fade, b.fade, r),
    };
    // Big mutation steps for real generational variance.
    g.aggr = (g.aggr + r.range(-0.08, 0.08)).clamp(-0.20, 0.20);
    g.risk = (g.risk + r.range(-0.25, 0.25)).clamp(0.0, 1.0);
    g.conf = (g.conf + r.range(-0.06, 0.06)).clamp(-0.15, 0.15);
    g.select = (g.select + r.range(-0.25, 0.25)).clamp(0.0, 1.0);
    g.press = (g.press + r.range(-0.25, 0.25)).clamp(0.0, 1.0);
    g.fade = (g.fade + r.range(-0.25, 0.25)).clamp(0.0, 1.0);
    g
}

fn name(r: &mut Rng) -> String {
    let a = NAMES[(r.next() as usize) % NAMES.len()];
    let b = NAMES[(r.next() as usize) % NAMES.len()];
    let suffix = format!("{:02X}", (r.next() % 256) as u8);
    format!("{a}-{b}-{suffix}")
}

pub fn load() -> Bloodline {
    std::fs::read_to_string(PATH).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

impl Bloodline {
    /// The genes driving the live oracle: the fittest living organism.
    pub fn champion_genes(&self) -> Genes {
        self.population
            .iter()
            .filter(|o| o.alive)
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal))
            .map(|o| o.genes.clone())
            .unwrap_or_default()
    }

    /// One day of selection: score everyone, cull the broke, breed the rich,
    /// then persist. Disk write is isolated in `save` so the core is testable.
    pub fn evolve(&mut self, date: &str, resolved: &[Prediction]) {
        self.evolve_in_memory(date, resolved);
        self.save();
    }

    /// The selection logic with no IO (so tests can exercise idempotency).
    pub fn evolve_in_memory(&mut self, date: &str, resolved: &[Prediction]) {
        let mut rng = Rng(ghash(date) ^ self.next_id.wrapping_mul(2654435761).wrapping_add(1));

        if self.population.is_empty() {
            for _ in 0..TARGET {
                let id = self.next_id;
                self.next_id += 1;
                let g = random_genes(&mut rng);
                self.population.push(Organism {
                    id, name: name(&mut rng), born: date.to_string(), parents: vec![],
                    genes: g, fitness: 1000.0, best: 1000.0, age: 0, alive: true,
                    died: String::new(), ..Default::default()
                });
            }
            self.gen = 1;
            self.last_evolved = date.to_string();
            // fall through so the founding generation gets a real stat line on
            // the same run (the once-per-day guard below still skips the cull).
        }

        let calls: Vec<(f64, bool)> = resolved
            .iter()
            .filter_map(|p| match p.status.as_str() {
                "HIT" => Some((conf_of(p), true)),
                "MISS" => Some((conf_of(p), false)),
                _ => None,
            })
            .collect();

        // The full stat line is recomputed every run (idempotent: same record,
        // same numbers). career-high `best` only ratchets up.
        for o in self.population.iter_mut().filter(|o| o.alive) {
            let s = simulate(&o.genes, &calls, o.id);
            o.fitness = s.bank;
            o.bets = s.bets;
            o.wins = s.wins;
            o.losses = s.losses;
            o.win_rate = if s.bets > 0 { s.wins * 100 / s.bets } else { 0 };
            o.max_streak = s.max_streak;
            o.biggest = s.biggest.round() as i64;
            o.big_bet = s.big_bet.round() as i64;
            o.roi = ((s.bank - 1000.0) / 10.0).round() as i64;
            if s.peak > o.best {
                o.best = s.peak;
            }
        }

        // The Hall of Fame: the all-time greats by career-high bankroll, kept
        // forever even after they are pruned from the living population. Updated
        // every run (idempotent: `best` only ratchets up).
        let snapshot: Vec<Organism> = self.population.iter().cloned().collect();
        for o in snapshot {
            if let Some(e) = self.hall_of_fame.iter_mut().find(|h| h.id == o.id) {
                *e = o;
            } else {
                self.hall_of_fame.push(o);
            }
        }
        self.hall_of_fame.sort_by(|a, b| b.best.partial_cmp(&a.best).unwrap_or(std::cmp::Ordering::Equal));
        self.hall_of_fame.truncate(12);

        // Aging, culling and breeding happen at most once per calendar day, so a
        // redundant cron run (a backstop firing) never over-ages or over-breeds.
        if self.last_evolved == date {
            return;
        }
        for o in self.population.iter_mut().filter(|o| o.alive) {
            o.age += 1;
        }
        self.last_evolved = date.to_string();

        // Cull and breed only once there is a real record to judge on.
        if calls.len() >= JUDGE_MIN {
            let mut alive: Vec<usize> = (0..self.population.len()).filter(|&i| self.population[i].alive).collect();
            alive.sort_by(|&a, &b| {
                self.population[b].fitness.partial_cmp(&self.population[a].fitness).unwrap_or(std::cmp::Ordering::Equal)
            });
            for &i in alive.iter().skip(SURVIVORS) {
                self.population[i].alive = false;
                self.population[i].died = date.to_string();
            }
            let survivors: Vec<usize> = alive.into_iter().take(SURVIVORS).collect();
            if !survivors.is_empty() {
                let need = TARGET.saturating_sub(survivors.len());
                for k in 0..need {
                    let pa = survivors[k % survivors.len()];
                    let pb = survivors[(k + 1) % survivors.len()];
                    let g = crossover(&self.population[pa].genes, &self.population[pb].genes, &mut rng);
                    let parents = vec![self.population[pa].id, self.population[pb].id];
                    let id = self.next_id;
                    self.next_id += 1;
                    let nm = name(&mut rng);
                    self.population.push(Organism {
                        id, name: nm, born: date.to_string(), parents, genes: g,
                        fitness: 1000.0, best: 1000.0, age: 0, alive: true,
                        died: String::new(), ..Default::default()
                    });
                }
            }
            self.gen += 1;

            // Prune the graveyard so the file stays bounded.
            let dead: Vec<usize> = (0..self.population.len()).filter(|&i| !self.population[i].alive).collect();
            if dead.len() > KEEP_DEAD {
                let drop: std::collections::HashSet<usize> = dead.into_iter().take(self.population.len()).rev().skip(KEEP_DEAD).collect();
                let mut idx = 0;
                self.population.retain(|_| { let keep = !drop.contains(&idx); idx += 1; keep });
            }
        }
    }

    fn save(&self) {
        if let Some(parent) = std::path::Path::new(PATH).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(j) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(PATH, j);
        }
    }

    /// The view the broadcast dramatizes: champion, the living ranked, the recent
    /// dead, the rival houses, and the newborns.
    pub fn to_json(&self) -> serde_json::Value {
        let mut living: Vec<&Organism> = self.population.iter().filter(|o| o.alive).collect();
        living.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));
        let mut dead: Vec<&Organism> = self.population.iter().filter(|o| !o.alive).collect();
        dead.sort_by(|a, b| b.died.cmp(&a.died));

        let org = |o: &Organism| {
            serde_json::json!({
                "id": o.id, "name": o.name, "born": o.born, "age": o.age,
                "fitness": o.fitness.round() as i64, "best": o.best.round() as i64,
                "parents": o.parents, "died": o.died,
                "house": house(&o.genes),
                "bets": o.bets, "wins": o.wins, "losses": o.losses,
                "win_rate": o.win_rate, "max_streak": o.max_streak,
                "biggest": o.biggest, "big_bet": o.big_bet, "roi": o.roi,
                "aggr": (o.genes.aggr * 100.0).round() / 100.0,
                "risk": (o.genes.risk * 100.0).round() / 100.0,
                "select": (o.genes.select * 100.0).round() as i64,
                "press": (o.genes.press * 100.0).round() as i64,
                "fade": if o.genes.fade > 0.5 { "FADE" } else { "TAIL" },
            })
        };

        // The rival houses: emergent lineages by temperament, racing for the den.
        let mut houses: std::collections::BTreeMap<&str, (i64, f64)> = std::collections::BTreeMap::new();
        for o in &living {
            let e = houses.entry(house(&o.genes)).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += o.fitness;
        }
        let mut house_rows: Vec<serde_json::Value> = houses
            .into_iter()
            .map(|(name, (n, fit))| serde_json::json!({ "name": name, "count": n, "fitness": fit.round() as i64 }))
            .collect();
        house_rows.sort_by(|a, b| b["fitness"].as_i64().unwrap_or(0).cmp(&a["fitness"].as_i64().unwrap_or(0)));

        let newborns: Vec<serde_json::Value> = living.iter().filter(|o| o.age == 0).map(|o| org(o)).collect();

        // ROOKIES: the most promising young blood (alive, age <= 3), by current
        // bankroll. PROS: the highest career stat lines among the living, by best.
        let mut rookies: Vec<&Organism> = living.iter().filter(|o| o.age <= 3).copied().collect();
        rookies.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));
        let mut pros: Vec<&Organism> = living.clone();
        pros.sort_by(|a, b| b.best.partial_cmp(&a.best).unwrap_or(std::cmp::Ordering::Equal));

        serde_json::json!({
            "gen": self.gen,
            "living_count": living.len(),
            "total_ever": self.next_id,
            "champion": living.first().map(|o| org(o)),
            "runner_up": living.get(1).map(|o| org(o)),
            "living": living.iter().map(|o| org(o)).collect::<Vec<_>>(),
            "dead": dead.iter().take(12).map(|o| org(o)).collect::<Vec<_>>(),
            "houses": house_rows,
            "newborns": newborns,
            "rookies": rookies.iter().take(4).map(|o| org(o)).collect::<Vec<_>>(),
            "pros": pros.iter().take(4).map(|o| org(o)).collect::<Vec<_>>(),
            "hall_of_fame": self.hall_of_fame.iter().take(10).map(|o| org(o)).collect::<Vec<_>>(),
        })
    }
}

/// The house an organism belongs to, emergent from its temperament (its genes).
fn house(g: &Genes) -> &'static str {
    if g.risk >= 0.62 {
        "THE PLUNGERS"
    } else if g.risk <= 0.34 {
        "THE MISERS"
    } else {
        "THE STEADY"
    }
}

#[cfg(test)]
#[path = "tests_bloodline.rs"]
mod tests_bloodline;
