// Publish per-subscriber EARLY FEED files, fully static (no Cloudflare, no KV).
//
// For each active Stripe subscription that carries a press credential (passed as
// client_reference_id at checkout), encrypt today's embargoed payload with a key
// derived from that credential and write docs/edge/<sha256(credential)>.json.
// The file is committed to the public repo but is ciphertext -- only the holder
// of the credential can decrypt it in the browser. Canceled subscribers stop
// getting a file published, so access is revoked on the next daily run.
//
// Needs: STRIPE_SECRET_KEY (env), build/early_payload.json (today's embargoed calls).

import { createHash, pbkdf2Sync, randomBytes, createCipheriv } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, rmSync } from "node:fs";

const KEY = process.env.STRIPE_SECRET_KEY;
const EDGE_DIR = "docs/edge";

if (!KEY) { console.log("publish_edge: no STRIPE_SECRET_KEY; skipping"); process.exit(0); }
if (!existsSync("build/early_payload.json")) { console.log("publish_edge: no early_payload.json; skipping"); process.exit(0); }

const payload = readFileSync("build/early_payload.json", "utf8");

function norm(s) { return (s || "").toUpperCase().replace(/[^A-Z0-9]+/g, "-").replace(/^-+|-+$/g, ""); }
function sha256hex(s) { return createHash("sha256").update(s).digest("hex"); }

function encrypt(plaintext, cred) {
  const salt = randomBytes(16);
  const iv = randomBytes(12);
  const key = pbkdf2Sync(cred, salt, 100000, 32, "sha256");
  const c = createCipheriv("aes-256-gcm", key, iv);
  const enc = Buffer.concat([c.update(plaintext, "utf8"), c.final()]);
  const ct = Buffer.concat([enc, c.getAuthTag()]); // WebCrypto expects ciphertext||tag
  return { v: 1, salt: salt.toString("base64"), iv: iv.toString("base64"), ct: ct.toString("base64") };
}

async function stripe(path) {
  const res = await fetch("https://api.stripe.com/v1/" + path, { headers: { Authorization: "Bearer " + KEY } });
  if (!res.ok) throw new Error("stripe " + path + " -> " + res.status);
  return res.json();
}

const active = new Set();
try {
  const sessions = await stripe("checkout/sessions?limit=100&expand[]=data.subscription");
  for (const s of sessions.data || []) {
    const cred = norm(s.client_reference_id);
    const sub = s.subscription;
    const ok = cred && sub && typeof sub === "object" && ["active", "trialing"].includes(sub.status);
    if (ok) active.add(cred);
  }
} catch (e) {
  console.log("publish_edge: stripe list failed (" + e.message + "); leaving edge dir untouched");
  process.exit(0);
}

mkdirSync(EDGE_DIR, { recursive: true });
const keep = new Set();
for (const cred of active) {
  const name = sha256hex(cred) + ".json";
  keep.add(name);
  writeFileSync(EDGE_DIR + "/" + name, JSON.stringify(encrypt(payload, cred)));
}
// Revoke: remove files for credentials that are no longer active.
for (const f of readdirSync(EDGE_DIR)) {
  if (f.endsWith(".json") && !keep.has(f)) rmSync(EDGE_DIR + "/" + f);
}
console.log("publish_edge: published " + keep.size + " subscriber feed(s)");
