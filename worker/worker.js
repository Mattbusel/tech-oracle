// SIGNAL / ORACLE -- edge access + accounts Worker (Cloudflare Workers + KV).
//
// THE ACCOUNT IS A PRESS CREDENTIAL. No email, no password, no username.
// Anyone can mint one for free; subscribing upgrades the same credential to
// premium (the early feed). The credential is the whole account.
//
//   GET  /new              -- mint a FREE credential (no payment) and show it.
//   GET  /?session_id=...  -- back from Stripe: upgrade your credential to premium.
//   GET  /?cred=...        -- log in with a credential (free or premium).
//   GET  /                 -- cookie -> early feed (premium) / account (free) / gate.
//   GET  /logout           -- clear the cookie.
//   POST /webhook          -- Stripe lifecycle -> KV active flags + downgrade.
//
// KV keys: acct:<CRED> = {tier, customer?}   sub:<customer> = active
//          cust:<customer> = <CRED>          edge_payload = today's embargoed JSON
// Secrets: STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, COOKIE_SECRET
// Vars: PAYMENT_LINK_URL, PORTAL_URL

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
    if (url.pathname === "/new") return handleNew(req, env);
    if (url.pathname === "/" || url.pathname === "/early") return handleGate(req, env, url);
    return new Response("Not found", { status: 404 });
  },
};

// --------------------------------------------------------------------------
// Free account creation
// --------------------------------------------------------------------------
async function handleNew(req, env) {
  const cred = randomCred();
  await env.KV.put("acct:" + cred, JSON.stringify({ tier: "free" }), { expirationTtl: 60 * 60 * 24 * 400 });
  return html(credentialPage(cred, "free", env), 200, await mintCookie(cred, env.COOKIE_SECRET));
}

// --------------------------------------------------------------------------
// Gate / login / checkout return
// --------------------------------------------------------------------------
async function handleGate(req, env, url) {
  // A) Back from Stripe checkout -> upgrade (or issue) a premium credential.
  const sessionId = url.searchParams.get("session_id");
  if (sessionId) {
    const session = await stripeGet(`checkout/sessions/${sessionId}`, env);
    const customer = session && (typeof session.customer === "string" ? session.customer : null);
    const paid = session && (session.status === "complete" || session.payment_status === "paid");
    if (customer && paid) {
      const existing = await credFromCookie(req, env.COOKIE_SECRET);
      const cred = existing || (await mintCredential(customer, env.COOKIE_SECRET));
      await env.KV.put("acct:" + cred, JSON.stringify({ tier: "premium", customer }), { expirationTtl: 60 * 60 * 24 * 400 });
      await env.KV.put("cust:" + customer, cred, { expirationTtl: 60 * 60 * 24 * 400 });
      await env.KV.put("sub:" + customer, JSON.stringify({ status: "active" }), { expirationTtl: 60 * 60 * 24 * 40 });
      return html(credentialPage(cred, "premium", env), 200, await mintCookie(cred, env.COOKIE_SECRET));
    }
    return gatePage(env, "We could not confirm that checkout. If you just paid, wait a moment and refresh.");
  }

  // B) Log in with a credential.
  const credParam = (url.searchParams.get("cred") || "").trim();
  if (credParam) {
    const cred = normalizeCred(credParam);
    const acct = await env.KV.get("acct:" + cred);
    if (acct) return new Response(null, { status: 302, headers: { Location: "/", "Set-Cookie": await mintCookie(cred, env.COOKIE_SECRET) } });
    return gatePage(env, "No account found for that credential. Check it, or create a free one.");
  }

  // C) Returning visitor.
  const cred = await credFromCookie(req, env.COOKIE_SECRET);
  if (!cred) return gatePage(env);
  const acct = JSON.parse((await env.KV.get("acct:" + cred)) || "null");
  if (!acct) return gatePage(env);
  const active = acct.tier === "premium" && acct.customer && (await env.KV.get("sub:" + acct.customer));
  if (active) {
    const payload = await env.KV.get("edge_payload");
    return html(earlyPage(payload ? JSON.parse(payload) : null, env));
  }
  return html(accountPage(cred, env)); // signed in, free tier
}

function logout() {
  return new Response(null, { status: 302, headers: { Location: "/", "Set-Cookie": "edge=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0" } });
}

// --------------------------------------------------------------------------
// Stripe webhook
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
    if (active) {
      await env.KV.put("sub:" + customer, JSON.stringify({ status: obj.status }), { expirationTtl: 60 * 60 * 24 * 40 });
    } else {
      await env.KV.delete("sub:" + customer);
      const cred = await env.KV.get("cust:" + customer); // downgrade the credential, keep the account
      if (cred) await env.KV.put("acct:" + cred, JSON.stringify({ tier: "free", customer }), { expirationTtl: 60 * 60 * 24 * 400 });
    }
  } else if (event.type === "invoice.payment_failed" && obj.customer) {
    await env.KV.delete("sub:" + obj.customer);
  }
  return new Response("ok");
}

// --------------------------------------------------------------------------
// Credential + crypto
// --------------------------------------------------------------------------
function credFromWords(hex, suffixFrom) {
  const w = [];
  for (let i = 0; i < 3; i++) w.push(WORDS[parseInt(hex.substr(i * 2, 2), 16) % WORDS.length]);
  return w.join("-") + "-" + hex.substr(suffixFrom, 4).toUpperCase();
}
function randomCred() {
  const b = new Uint8Array(8); crypto.getRandomValues(b);
  const hex = [...b].map((x) => x.toString(16).padStart(2, "0")).join("");
  return credFromWords(hex, 12);
}
async function mintCredential(customer, secret) {
  return credFromWords(await hmacHex(secret, "cred:" + customer), 60);
}
function normalizeCred(s) { return s.toUpperCase().replace(/[^A-Z0-9]+/g, "-").replace(/^-+|-+$/g, ""); }
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
async function mintCookie(cred, secret) {
  const exp = Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 30;
  const data = cred + "." + exp;
  return "edge=" + b64url(data) + "." + (await hmacHex(secret, data)) + "; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=" + 60 * 60 * 24 * 30;
}
async function credFromCookie(req, secret) {
  const c = (req.headers.get("Cookie") || "").split(";").map((s) => s.trim()).find((s) => s.startsWith("edge="));
  if (!c) return null;
  const [d, sig] = c.slice(5).split(".");
  if (!d || !sig) return null;
  const data = unb64url(d);
  if (!timingSafeEqual(await hmacHex(secret, data), sig)) return null;
  const idx = data.lastIndexOf(".");
  const cred = data.slice(0, idx), exp = data.slice(idx + 1);
  return Number(exp) < Math.floor(Date.now() / 1000) ? null : cred;
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
.band{display:inline-block;background:var(--ink);color:var(--paper);font-weight:600;font-size:12px;letter-spacing:.2em;padding:4px 12px;margin-bottom:16px}
.cred{font-size:clamp(20px,5vw,30px);font-weight:700;letter-spacing:.08em;border:2px dashed var(--ink);padding:18px;text-align:center;margin:6px 0 10px;word-break:break-word}
.note{font-size:12px;line-height:1.6;color:#3b3a30;margin:12px 0}.soft{color:var(--soft);font-size:11px;letter-spacing:.06em}
input{width:100%;font-family:inherit;font-size:16px;letter-spacing:.1em;text-transform:uppercase;padding:13px;border:1.5px solid var(--ink);background:#fff;color:var(--ink);margin:8px 0}
.btn{display:inline-block;width:100%;text-align:center;background:var(--ink);color:var(--paper);font-weight:600;font-size:13px;letter-spacing:.12em;text-decoration:none;padding:14px;border:none;cursor:pointer;margin-top:8px}
.btn.alt{background:none;color:var(--ink);border:1.5px solid var(--ink)}
a{color:var(--ink)}.err{color:var(--stamp);font-size:12px;margin:10px 0}.call{margin:14px 0;padding-bottom:12px;border-bottom:1.5px dashed var(--rule)}
.call .meta{font-size:11px;color:var(--soft);letter-spacing:.08em}.call p{font-size:16px;margin:6px 0}.hr{border-top:1.5px dashed var(--rule);margin:22px 0}
</style></head><body><div class="sheet">${inner}</div></body></html>`;

function credentialPage(cred, tier, env) {
  const premium = tier === "premium";
  return `<h1>THE SIGNAL</h1><div class="tag">PRESS CREDENTIAL ISSUED</div>
  <div class="band">${premium ? "PREMIUM KEY" : "YOUR FREE KEY"}</div>
  <div class="cred">${esc(cred)}</div>
  <p class="note"><b>Save this now.</b> It is your account and your only key. There is no email or password to recover it.</p>
  ${premium
      ? `<a class="btn" href="/">ENTER THE EARLY FEED</a>`
      : `<p class="note">This is a free account. Subscribe to unlock the early feed (every call before it prints publicly); your credential simply upgrades in place.</p><a class="btn" href="${env.PAYMENT_LINK_URL}">SUBSCRIBE TO UPGRADE</a><a class="btn alt" href="/">CONTINUE</a>`}
  <p class="soft" style="margin-top:18px"><a href="${env.PORTAL_URL}">Manage subscription</a></p>`;
}

function gatePage(env, error) {
  const inner = `<h1>THE SIGNAL</h1><div class="tag">CREDENTIAL DESK</div>
  ${error ? `<div class="err">${esc(error)}</div>` : ""}
  <div class="band">ENTER YOUR PRESS CREDENTIAL</div>
  <form method="get" action="/">
    <input name="cred" placeholder="RIBBON-COPPER-VECTOR-7F3A" autocomplete="off" autocapitalize="characters" spellcheck="false">
    <button class="btn" type="submit">SIGN IN</button>
  </form>
  <div class="hr"></div>
  <div class="band">NO CREDENTIAL YET</div>
  <p class="note">Anyone can mint one. A free credential is your account; subscribing upgrades the same credential to the early feed.</p>
  <a class="btn alt" href="/new">CREATE A FREE PRESS CREDENTIAL</a>
  <a class="btn" href="${env.PAYMENT_LINK_URL}">SUBSCRIBE -- GET A PREMIUM CREDENTIAL</a>`;
  return html(inner, error ? 200 : 200);
}

function accountPage(cred, env) {
  return `<h1>THE SIGNAL</h1><div class="tag">SIGNED IN // FREE TIER</div>
  <div class="band">YOUR CREDENTIAL</div>
  <div class="cred">${esc(cred)}</div>
  <p class="note">You have a free press credential. The early feed (every call before it prints publicly) is for subscribers. Subscribe and this same credential upgrades in place.</p>
  <a class="btn" href="${env.PAYMENT_LINK_URL}">SUBSCRIBE TO UNLOCK THE EARLY FEED</a>
  <p class="soft" style="margin-top:18px"><a href="/logout">Sign out</a></p>`;
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
