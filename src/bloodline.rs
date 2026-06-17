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
const TARGET: usize = 10; // living population size
const SURVIVORS: usize = 7; // how many live through each cull
const KEEP_DEAD: usize = 40; // graveyard depth
const JUDGE_MIN: usize = 5; // settled calls needed before any culling

const NAMES: &[&str] = &[
    "RIBBON", "COPPER", "VECTOR", "CIPHER", "EMBER", "QUARTZ", "PISTON", "LANTERN", "STYLUS",
    "CARBON", "RELAY", "DELTA", "OXIDE", "PRISM", "ANVIL", "COBALT", "HALIDE", "INDIGO", "KELVIN",
    "LUMEN", "MAGNET", "ONYX", "QUASAR", "ROTOR", "SOLDER", "ULTRA", "VOLT", "WAFER", "XENON",
    "ZINC", "BRASS", "CEDAR", "DRIFT", "FLINT", "GLASS", "IRON", "JADE", "NOVA", "PEARL", "SLATE",
];

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Genes {
    pub aggr: f64, // line aggressiveness (shifts confidence)
    pub risk: f64, // longshot appetite (stake variance)
    pub conf: f64, // baseline confidence bias
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Organism {
    pub id: u64,
    pub name: String,
    pub born: String,
    pub parents: Vec<u64>,
    pub genes: Genes,
    pub fitness: f64,
    pub age: i64,
    pub alive: bool,
    pub died: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Bloodline {
    pub next_id: u64,
    pub gen: i64,
    pub population: Vec<Organism>,
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

/// One organism's shadow bankroll over the settled record, betting each call
/// with a stake and line set by its own genes. This is its fitness.
fn simulate(g: &Genes, calls: &[(f64, bool)]) -> f64 {
    let mut bank = 1000.0_f64;
    for (conf, hit) in calls {
        let c = (conf + g.aggr + g.conf).clamp(0.34, 0.95);
        let stake = (100.0 * (1.0 + g.risk * (1.5 - c))).max(20.0);
        if *hit {
            bank += stake * ((1.0 / c) - 1.0);
        } else {
            bank -= stake;
        }
    }
    bank
}

fn random_genes(r: &mut Rng) -> Genes {
    Genes { aggr: r.range(-0.10, 0.10), risk: r.range(0.0, 1.0), conf: r.range(-0.05, 0.05) }
}

fn crossover(a: &Genes, b: &Genes, r: &mut Rng) -> Genes {
    let pick = |x: f64, y: f64, r: &mut Rng| if r.unit() < 0.5 { x } else { y };
    let mut g = Genes {
        aggr: pick(a.aggr, b.aggr, r),
        risk: pick(a.risk, b.risk, r),
        conf: pick(a.conf, b.conf, r),
    };
    // mutation
    g.aggr = (g.aggr + r.range(-0.03, 0.03)).clamp(-0.12, 0.12);
    g.risk = (g.risk + r.range(-0.12, 0.12)).clamp(0.0, 1.0);
    g.conf = (g.conf + r.range(-0.02, 0.02)).clamp(-0.08, 0.08);
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

    /// One day of selection: score everyone, cull the broke, breed the rich.
    pub fn evolve(&mut self, date: &str, resolved: &[Prediction]) {
        let mut rng = Rng(ghash(date) ^ self.next_id.wrapping_mul(2654435761).wrapping_add(1));

        if self.population.is_empty() {
            for _ in 0..TARGET {
                let id = self.next_id;
                self.next_id += 1;
                let g = random_genes(&mut rng);
                self.population.push(Organism {
                    id, name: name(&mut rng), born: date.to_string(), parents: vec![],
                    genes: g, fitness: 1000.0, age: 0, alive: true, died: String::new(),
                });
            }
            self.gen = 1;
            self.save();
            return;
        }

        let calls: Vec<(f64, bool)> = resolved
            .iter()
            .filter_map(|p| match p.status.as_str() {
                "HIT" => Some((conf_of(p), true)),
                "MISS" => Some((conf_of(p), false)),
                _ => None,
            })
            .collect();

        for o in self.population.iter_mut().filter(|o| o.alive) {
            o.fitness = simulate(&o.genes, &calls);
            o.age += 1;
        }

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
                        fitness: 1000.0, age: 0, alive: true, died: String::new(),
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

        self.save();
    }

    fn save(&self) {
        if let Some(parent) = std::path::Path::new(PATH).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(j) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(PATH, j);
        }
    }

    /// The view the page dramatizes: champion, the living ranked, the recent dead.
    pub fn to_json(&self) -> serde_json::Value {
        let mut living: Vec<&Organism> = self.population.iter().filter(|o| o.alive).collect();
        living.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));
        let mut dead: Vec<&Organism> = self.population.iter().filter(|o| !o.alive).collect();
        dead.sort_by(|a, b| b.died.cmp(&a.died));

        let org = |o: &Organism| {
            serde_json::json!({
                "id": o.id, "name": o.name, "born": o.born, "age": o.age,
                "fitness": o.fitness.round() as i64, "parents": o.parents, "died": o.died,
                "aggr": (o.genes.aggr * 100.0).round() / 100.0,
                "risk": (o.genes.risk * 100.0).round() / 100.0,
            })
        };
        serde_json::json!({
            "gen": self.gen,
            "living_count": living.len(),
            "total_ever": self.next_id,
            "champion": living.first().map(|o| org(o)),
            "living": living.iter().map(|o| org(o)).collect::<Vec<_>>(),
            "dead": dead.iter().take(12).map(|o| org(o)).collect::<Vec<_>>(),
        })
    }
}
