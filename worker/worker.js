// SIGNAL · ORACLE — edge access Worker (Cloudflare Workers + KV, free tier).
//
// This is the ONLY backend in the project, and it does the minimum:
//   1. POST /webhook  — verify Stripe webhook signatures, keep a tiny KV record
//                       of which customers are active subscribers.
//   2. GET  /         — gate the early payload. A fresh checkout (?session_id=)
//                       is verified against Stripe, mints a signed cookie, and
//                       is marked active in KV. Returning visitors are checked
//                       by cookie + KV. Active subscribers get the early calls;
//                       everyone else gets a paywall.
//
// We hold NO card data and run NO database: Stripe hosts checkout/billing, KV
// (free tier) holds only `sub:<customer>` flags and the daily `edge_payload`.
//
// Secrets (set with `wrangler secret put <NAME>`):
//   STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, COOKIE_SECRET
// Vars (wrangler.toml [vars]): PAYMENT_LINK_URL, PORTAL_URL
// KV binding: KV   (holds `sub:<customer>` and `edge_payload`)

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method === "POST" && url.pathname === "/webhook") return handleWebhook(req, env);
    if (url.pathname === "/logout") return logout();
    if (url.pathname === "/" || url.pathname === "/early") return handleEarly(req, env, url);
    return new Response("Not found", { status: 404 });
  },
};

// --------------------------------------------------------------------------
// Stripe webhook: keep KV in sync with subscription lifecycle.
// --------------------------------------------------------------------------
async function handleWebhook(req, env) {
  const body = await req.text();
  const sig = req.headers.get("stripe-signature") || "";
  if (!(await verifyStripeSignature(body, sig, env.STRIPE_WEBHOOK_SECRET))) {
    return new Response("invalid signature", { status: 400 });
  }

  const event = JSON.parse(body);
  const obj = event.data && event.data.object;

  if (event.type.startsWith("customer.subscription.")) {
    const customer = obj.customer;
    const deleted = event.type === "customer.subscription.deleted";
    const active = !deleted && ["active", "trialing"].includes(obj.status);
    if (active) {
      // 40-day TTL is a backstop; renewals refresh it well before expiry.
      await env.KV.put(`sub:${customer}`, JSON.stringify({ status: obj.status }), {
        expirationTtl: 60 * 60 * 24 * 40,
      });
    } else {
      await env.KV.delete(`sub:${customer}`);
    }
  } else if (event.type === "invoice.payment_failed") {
    // Optional hard cut on failed payment; dunning still runs in Stripe.
    if (obj.customer) await env.KV.delete(`sub:${obj.customer}`);
  }

  return new Response("ok");
}

// --------------------------------------------------------------------------
// Gated early payload.
// --------------------------------------------------------------------------
async function handleEarly(req, env, url) {
  // Path A: just came back from Stripe Checkout.
  const sessionId = url.searchParams.get("session_id");
  if (sessionId) {
    const session = await stripeGet(`checkout/sessions/${sessionId}`, env);
    const customer = session && (typeof session.customer === "string" ? session.customer : null);
    const paid = session && (session.status === "complete" || session.payment_status === "paid");
    if (customer && paid) {
      await env.KV.put(`sub:${customer}`, JSON.stringify({ status: "active", via: "checkout" }), {
        expirationTtl: 60 * 60 * 24 * 40,
      });
      const cookie = await mintCookie(customer, env.COOKIE_SECRET);
      return new Response(null, {
        status: 302,
        headers: {
          Location: url.pathname,
          "Set-Cookie": `edge=${cookie}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=${60 * 60 * 24 * 30}`,
        },
      });
    }
    return paywall(env, "We couldn't confirm that checkout. If you just paid, give it a few seconds and refresh.");
  }

  // Path B: returning visitor — verify cookie, then KV.
  const customer = await customerFromCookie(req, env.COOKIE_SECRET);
  const active = customer && (await env.KV.get(`sub:${customer}`));
  if (!active) return paywall(env);

  const payload = await env.KV.get("edge_payload");
  if (!payload) return earlyPage(null, env);
  return earlyPage(JSON.parse(payload), env);
}

function logout() {
  return new Response(null, {
    status: 302,
    headers: { Location: "/", "Set-Cookie": "edge=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0" },
  });
}

// --------------------------------------------------------------------------
// Stripe helpers
// --------------------------------------------------------------------------
async function stripeGet(path, env) {
  const res = await fetch(`https://api.stripe.com/v1/${path}`, {
    headers: { Authorization: `Bearer ${env.STRIPE_SECRET_KEY}` },
  });
  if (!res.ok) return null;
  return res.json();
}

// Verify `Stripe-Signature: t=...,v1=...` per Stripe's scheme: HMAC-SHA256 of
// `${t}.${body}` keyed by the webhook secret, with a timestamp tolerance.
async function verifyStripeSignature(body, header, secret) {
  const parts = Object.fromEntries(header.split(",").map((kv) => kv.split("=")));
  const t = parts.t;
  const v1 = parts.v1;
  if (!t || !v1) return false;
  if (Math.abs(Date.now() / 1000 - Number(t)) > 300) return false; // 5-min tolerance
  const expected = await hmacHex(secret, `${t}.${body}`);
  return timingSafeEqual(expected, v1);
}

// --------------------------------------------------------------------------
// Cookie minting / verification (signed, no server session store)
// --------------------------------------------------------------------------
async function mintCookie(customer, secret) {
  const exp = Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 30;
  const data = `${customer}.${exp}`;
  const sig = await hmacHex(secret, data);
  return `${b64url(data)}.${sig}`;
}

async function customerFromCookie(req, secret) {
  const cookie = (req.headers.get("Cookie") || "").split(";").map((c) => c.trim()).find((c) => c.startsWith("edge="));
  if (!cookie) return null;
  const raw = cookie.slice("edge=".length);
  const [dataB64, sig] = raw.split(".");
  if (!dataB64 || !sig) return null;
  const data = unb64url(dataB64);
  if (!(await hmacHex(secret, data).then((h) => timingSafeEqual(h, sig)))) return null;
  const [customer, exp] = data.split(".");
  if (Number(exp) < Math.floor(Date.now() / 1000)) return null;
  return customer;
}

// --------------------------------------------------------------------------
// Crypto utils (Web Crypto)
// --------------------------------------------------------------------------
async function hmacHex(secret, message) {
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  const buf = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(message));
  return [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

function timingSafeEqual(a, b) {
  if (a.length !== b.length) return false;
  let out = 0;
  for (let i = 0; i < a.length; i++) out |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return out === 0;
}

function b64url(s) {
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
function unb64url(s) {
  return atob(s.replace(/-/g, "+").replace(/_/g, "/"));
}

// --------------------------------------------------------------------------
// HTML responses
// --------------------------------------------------------------------------
const SHELL = (title, inner) => `<!doctype html><html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title}</title>
<style>
  :root{--bg:#0a0b1a;--text:#eef0fa;--muted:#9aa0bd;--cyan:#5ed6ff;--violet:#b69cff;--border:rgba(255,255,255,.12)}
  body{margin:0;font-family:Inter,system-ui,sans-serif;background:radial-gradient(900px 600px at 80% -10%,rgba(94,214,255,.12),transparent),linear-gradient(#0a0b1a,#10122b);color:var(--text);line-height:1.55;min-height:100vh}
  .wrap{max-width:760px;margin:0 auto;padding:48px 22px}
  .eyebrow{font-family:ui-monospace,monospace;letter-spacing:.3em;text-transform:uppercase;font-size:12px;color:var(--cyan)}
  h1{font-size:30px;letter-spacing:-.02em;margin:14px 0 8px}
  .card{background:rgba(255,255,255,.04);border:1px solid var(--border);border-radius:14px;padding:20px;margin:16px 0}
  .call{font-size:18px;margin:0 0 8px}
  time{font-family:ui-monospace,monospace;font-size:12px;color:var(--muted)}
  a{color:var(--cyan)}
  .btn{display:inline-block;margin-top:12px;padding:11px 18px;border-radius:11px;font-weight:600;text-decoration:none;color:#06121a;background:linear-gradient(100deg,var(--cyan),var(--violet))}
  .muted{color:var(--muted)} .reveal{font-size:12px;color:var(--muted);font-family:ui-monospace,monospace}
</style></head><body><div class="wrap">${inner}</div></body></html>`;

function earlyPage(payload, env) {
  let body;
  if (!payload || !payload.predictions || payload.predictions.length === 0) {
    body = `<p class="muted">No embargoed calls right now — every current call is already public. Sit tight; the next one drops on the daily run.</p>`;
  } else {
    body = payload.predictions
      .map(
        (p) => `<div class="card">
          <p class="call">${escapeHtml(p.prediction_text)}</p>
          <time>called ${escapeHtml(p.date)}</time> ·
          <span class="reveal">public on ${escapeHtml(p.public_reveal_date)}</span><br>
          <a href="${escapeHtml(p.source_url)}" rel="noopener">source ↗</a>
        </div>`
      )
      .join("");
  }
  const inner = `<div class="eyebrow">Subscriber edge</div>
    <h1>Tomorrow's calls, today</h1>
    <p class="muted">${payload ? escapeHtml(payload.count + "") : "0"} embargoed call(s). These are public ${payload ? payload.reveal_delay_days : ""} day(s) after they're called.</p>
    ${body}
    <p style="margin-top:28px"><a href="${env.PORTAL_URL}" rel="noopener">Manage subscription</a> · <a href="/logout">Sign out</a></p>`;
  return html(SHELL("Subscriber edge — SIGNAL", inner));
}

function paywall(env, note) {
  const inner = `<div class="eyebrow">Subscriber edge</div>
    <h1>This is the early feed.</h1>
    ${note ? `<p class="muted">${escapeHtml(note)}</p>` : ""}
    <p class="muted">Subscribers see each call days before it hits the public archive. Subscribe to unlock today's calls.</p>
    <a class="btn" href="${env.PAYMENT_LINK_URL}" rel="noopener">Subscribe →</a>
    <p style="margin-top:18px"><a href="${env.PORTAL_URL}" rel="noopener">Already subscribed? Manage / restore access</a></p>`;
  return html(SHELL("Subscribe — SIGNAL", inner), 402);
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}
function html(s, status = 200) {
  return new Response(s, { status, headers: { "content-type": "text/html; charset=utf-8", "cache-control": "no-store" } });
}
