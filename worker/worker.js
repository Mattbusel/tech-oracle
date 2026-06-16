// SIGNAL / ORACLE -- edge access Worker (Cloudflare Workers + KV, free tier).
//
// The account system is the PRESS CREDENTIAL: no email, no password, no
// username. On a successful Stripe checkout the press issues a unique,
// deterministic credential (e.g. RIBBON-COPPER-VECTOR-7F3A) derived from the
// Stripe customer id. That credential IS the account. You log in by entering
// it; a signed cookie remembers you after that. Lost it? Re-run checkout or the
// customer portal and the same credential is re-issued.
//
// Routes:
//   POST /webhook  -- verify Stripe webhook signatures; track active customers.
//   GET  /?session_id=...  -- back from checkout: mint + show the credential.
//   GET  /?cred=...        -- log in with a credential.
//   GET  /                 -- cookie -> early feed; else the credential gate.
//   GET  /logout           -- clear the cookie.
//
// Secrets (wrangler secret put): STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, COOKIE_SECRET
// Vars (wrangler.toml): PAYMENT_LINK_URL, PORTAL_URL
// KV binding: KV  (keys: sub:<customer>, cred:<CREDENTIAL>, edge_payload)

const WORDS = [
  "RIBBON", "PLATEN", "SPROCKET", "COPPER", "VECTOR", "CIPHER", "BEACON", "EMBER",
  "QUARTZ", "PISTON", "LANTERN", "MARGIN", "FOLIO", "GALLEY", "STYLUS", "CARBON",
  "TUNGSTEN", "RELAY", "DELTA", "OXIDE", "PRISM", "ANVIL", "COBALT", "FERRITE",
  "GASKET", "HALIDE", "INDIGO", "JUNCTION", "KELVIN", "LUMEN", "MAGNET", "NICKEL",
  "ONYX", "PHOSPHOR", "QUASAR", "ROTOR", "SOLDER", "TELEX", "ULTRA", "VOLT",
  "WAFER", "XENON", "YOKE", "ZINC", "ASHEN", "BRASS", "CEDAR", "DRIFT",
];

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method === "POST" && url.pathname === "/webhook") return handleWebhook(req, env);
    if (url.pathname === "/logout") return logout();
    if (url.pathname === "/" || url.pathname === "/early") return handleGate(req, env, url);
    return new Response("Not found", { status: 404 });
  },
};

// --------------------------------------------------------------------------
// Gate
// --------------------------------------------------------------------------
async function handleGate(req, env, url) {
  // A) Returning from Stripe checkout: mint and present the credential.
  const sessionId = url.searchParams.get("session_id");
  if (sessionId) {
    const session = await stripeGet(`checkout/sessions/${sessionId}`, env);
    const customer = session && (typeof session.customer === "string" ? session.customer : null);
    const paid = session && (session.status === "complete" || session.payment_status === "paid");
    if (customer && paid) {
      const cred = await mintCredential(customer, env.COOKIE_SECRET);
      await env.KV.put("cred:" + cred, customer, { expirationTtl: 60 * 60 * 24 * 400 });
      await env.KV.put("sub:" + customer, JSON.stringify({ status: "active", via: "checkout" }), { expirationTtl: 60 * 60 * 24 * 40 });
      const cookie = await mintCookie(customer, env.COOKIE_SECRET);
      return html(credentialPage(cred, env), 200, cookie);
    }
    return gatePage(env, "We could not confirm that checkout. If you just paid, wait a moment and refresh.");
  }

  // B) Logging in with a credential.
  const cred = (url.searchParams.get("cred") || "").trim();
  if (cred) {
    const norm = normalizeCred(cred);
    const customer = await env.KV.get("cred:" + norm);
    const active = customer && (await env.KV.get("sub:" + customer));
    if (active) {
      const cookie = await mintCookie(customer, env.COOKIE_SECRET);
      return new Response(null, { status: 302, headers: { Location: "/", "Set-Cookie": cookie } });
    }
    return gatePage(env, "That credential is not active. Check it, or re-subscribe to be re-issued one.");
  }

  // C) Returning visitor: cookie -> feed, else the gate.
  const customer = await customerFromCookie(req, env.COOKIE_SECRET);
  const ok = customer && (await env.KV.get("sub:" + customer));
  if (!ok) return gatePage(env);
  const payload = await env.KV.get("edge_payload");
  return html(earlyPage(payload ? JSON.parse(payload) : null, env));
}

function logout() {
  return new Response(null, { status: 302, headers: { Location: "/", "Set-Cookie": "edge=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0" } });
}

// --------------------------------------------------------------------------
// Stripe webhook -> KV active flags
// --------------------------------------------------------------------------
async function handleWebhook(req, env) {
  const body = await req.text();
  if (!(await verifyStripeSignature(body, req.headers.get("stripe-signature") || "", env.STRIPE_WEBHOOK_SECRET))) {
    return new Response("invalid signature", { status: 400 });
  }
  const event = JSON.parse(body);
  const obj = event.data && event.data.object;
  if (event.type.startsWith("customer.subscription.")) {
    const customer = obj.customer;
    const active = event.type !== "customer.subscription.deleted" && ["active", "trialing"].includes(obj.status);
    if (active) await env.KV.put("sub:" + customer, JSON.stringify({ status: obj.status }), { expirationTtl: 60 * 60 * 24 * 40 });
    else await env.KV.delete("sub:" + customer);
  } else if (event.type === "invoice.payment_failed" && obj.customer) {
    await env.KV.delete("sub:" + obj.customer);
  }
  return new Response("ok");
}

// --------------------------------------------------------------------------
// Credential + crypto
// --------------------------------------------------------------------------
async function mintCredential(customer, secret) {
  const hex = await hmacHex(secret, "cred:" + customer);
  const w = [];
  for (let i = 0; i < 3; i++) w.push(WORDS[parseInt(hex.substr(i * 2, 2), 16) % WORDS.length]);
  return w.join("-") + "-" + hex.substr(60, 4).toUpperCase();
}
function normalizeCred(s) {
  return s.toUpperCase().replace(/[^A-Z0-9]+/g, "-").replace(/^-+|-+$/g, "");
}
async function stripeGet(path, env) {
  const res = await fetch("https://api.stripe.com/v1/" + path, { headers: { Authorization: "Bearer " + env.STRIPE_SECRET_KEY } });
  return res.ok ? res.json() : null;
}
async function verifyStripeSignature(body, header, secret) {
  const parts = Object.fromEntries(header.split(",").map((kv) => kv.split("=")));
  if (!parts.t || !parts.v1) return false;
  if (Math.abs(Date.now() / 1000 - Number(parts.t)) > 300) return false;
  return timingSafeEqual(await hmacHex(secret, parts.t + "." + body), parts.v1);
}
async function mintCookie(customer, secret) {
  const exp = Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 30;
  const data = customer + "." + exp;
  const sig = await hmacHex(secret, data);
  return "edge=" + b64url(data) + "." + sig + "; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=" + 60 * 60 * 24 * 30;
}
async function customerFromCookie(req, secret) {
  const c = (req.headers.get("Cookie") || "").split(";").map((s) => s.trim()).find((s) => s.startsWith("edge="));
  if (!c) return null;
  const [d, sig] = c.slice(5).split(".");
  if (!d || !sig) return null;
  const data = unb64url(d);
  if (!timingSafeEqual(await hmacHex(secret, data), sig)) return null;
  const [customer, exp] = data.split(".");
  return Number(exp) < Math.floor(Date.now() / 1000) ? null : customer;
}
async function hmacHex(secret, message) {
  const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  const buf = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(message));
  return [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, "0")).join("");
}
function timingSafeEqual(a, b) { if (a.length !== b.length) return false; let o = 0; for (let i = 0; i < a.length; i++) o |= a.charCodeAt(i) ^ b.charCodeAt(i); return o === 0; }
function b64url(s) { return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, ""); }
function unb64url(s) { return atob(s.replace(/-/g, "+").replace(/_/g, "/")); }

// --------------------------------------------------------------------------
// HTML
// --------------------------------------------------------------------------
const SHELL = (title, inner) => `<!doctype html><html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${title}</title>
<link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>
:root{--paper:#efede4;--ink:#1b1a14;--soft:#6d6b5e;--stamp:#b23a2e;--rule:rgba(27,26,20,.28)}
*{box-sizing:border-box}body{margin:0;background:#17181c;color:var(--ink);font-family:'IBM Plex Mono',monospace}
.sheet{max-width:560px;margin:0 auto;background:var(--paper);min-height:100vh;padding:40px 40px 60px;
 background-image:radial-gradient(circle 3.4px,#17181c 96%,transparent 100%),radial-gradient(circle 3.4px,#17181c 96%,transparent 100%);
 background-size:22px 22px,22px 22px;background-position:14px 18px,calc(100% - 14px) 18px;background-repeat:repeat-y,repeat-y}
h1{font-size:30px;letter-spacing:.14em;margin:0}.tag{font-size:11px;letter-spacing:.4em;color:var(--soft);margin:8px 0 26px}
.band{display:inline-block;background:var(--ink);color:var(--paper);font-weight:600;font-size:12px;letter-spacing:.2em;padding:4px 12px;margin-bottom:18px}
.cred{font-size:clamp(20px,5vw,30px);font-weight:700;letter-spacing:.08em;border:2px dashed var(--ink);padding:18px;text-align:center;margin:6px 0 10px;word-break:break-word}
.note{font-size:12px;line-height:1.6;color:#3b3a30;margin:12px 0}.soft{color:var(--soft);font-size:11px;letter-spacing:.06em}
input{width:100%;font-family:inherit;font-size:16px;letter-spacing:.1em;text-transform:uppercase;padding:13px;border:1.5px solid var(--ink);background:#fff;color:var(--ink);margin:8px 0}
.btn{display:inline-block;width:100%;text-align:center;background:var(--ink);color:var(--paper);font-weight:600;font-size:13px;letter-spacing:.12em;text-decoration:none;padding:14px;border:none;cursor:pointer;margin-top:8px}
.btn.alt{background:none;color:var(--ink);border:1.5px solid var(--ink)}
a{color:var(--ink)}.err{color:var(--stamp);font-size:12px;margin:10px 0}.call{margin:14px 0;padding-bottom:12px;border-bottom:1.5px dashed var(--rule)}
.call .meta{font-size:11px;color:var(--soft);letter-spacing:.08em}.call p{font-size:16px;margin:6px 0}
</style></head><body><div class="sheet">${inner}</div></body></html>`;

function credentialPage(cred, env) {
  return `<h1>THE SIGNAL</h1><div class="tag">PRESS CREDENTIAL ISSUED</div>
  <div class="band">YOUR KEY</div>
  <div class="cred">${esc(cred)}</div>
  <p class="note"><b>Save this now.</b> It is your account and your only key to the early feed. No email or password exists to recover it; if you lose it, re-open checkout or the customer portal and the same credential is re-issued.</p>
  <a class="btn" href="/">ENTER THE EARLY FEED</a>
  <p class="soft" style="margin-top:18px"><a href="${env.PORTAL_URL}">Manage subscription</a></p>`;
}

function gatePage(env, error) {
  const inner = `<h1>THE SIGNAL</h1><div class="tag">SUBSCRIBER GATE</div>
  ${error ? `<div class="err">${esc(error)}</div>` : ""}
  <div class="band">ENTER YOUR PRESS CREDENTIAL</div>
  <form method="get" action="/">
    <input name="cred" placeholder="RIBBON-COPPER-VECTOR-7F3A" autocomplete="off" autocapitalize="characters" spellcheck="false">
    <button class="btn" type="submit">UNLOCK THE EARLY FEED</button>
  </form>
  <p class="note">No email. No password. Your press credential is the whole account.</p>
  <div class="band" style="margin-top:22px">NOT A SUBSCRIBER YET</div>
  <p class="note">Subscribers see every call before it prints on the public record. Subscribe and the press issues your credential on the spot.</p>
  <a class="btn alt" href="${env.PAYMENT_LINK_URL}">SUBSCRIBE -- GET THE LEAD</a>`;
  return html(inner, error ? 200 : 402);
}

function earlyPage(payload, env) {
  let body;
  if (!payload || !payload.predictions || !payload.predictions.length) {
    body = `<p class="note">No embargoed calls right now. Every current call is already public. The next drop arrives on the daily run.</p>`;
  } else {
    body = payload.predictions.map((p) => `<div class="call"><div class="meta">CALLED ${esc(p.date)} // PUBLIC ${esc(p.public_reveal_date)}</div><p>${esc(p.prediction_text)}</p><a class="soft" href="${esc(p.source_url)}">source</a></div>`).join("");
  }
  return `<h1>THE SIGNAL</h1><div class="tag">EARLY FEED // SUBSCRIBER</div>
  <div class="band">${payload ? esc(payload.count + "") : "0"} CALLS, AHEAD OF THE RECORD</div>
  ${body}
  <p class="soft" style="margin-top:20px"><a href="${env.PORTAL_URL}">Manage</a> &nbsp;//&nbsp; <a href="/logout">Sign out</a></p>`;
}

function esc(s) { return String(s == null ? "" : s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])); }
function html(inner, status, cookie) {
  const headers = { "content-type": "text/html; charset=utf-8", "cache-control": "no-store" };
  if (cookie) headers["Set-Cookie"] = cookie;
  return new Response(SHELL("THE SIGNAL", inner), { status: status || 200, headers });
}
