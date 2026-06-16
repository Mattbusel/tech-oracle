// Publish per-subscriber EARLY FEED files, fully static (no Cloudflare, no KV).
//
// For each active Stripe subscription that carries a press credential (passed as
// client_reference_id at checkout) AND for each comp "god pass" in COMP_CREDS,
// encrypt today's embargoed payload with a key derived from that credential and
// write docs/edge/<sha256(credential)>.json. The file is committed to the public
// repo but is ciphertext -- only the holder of the credential can decrypt it in
// the browser. Removing a subscriber (cancel) or a god pass (edit COMP_CREDS)
// stops its file being published, revoking access on the next daily run.
//
// Env: STRIPE_SECRET_KEY (optional), COMP_CREDS (optional, comma/space/newline
// separated god passes). Needs build/early_payload.json (today's embargoed calls).

import { createHash, pbkdf2Sync, randomBytes, createCipheriv } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, rmSync } from "node:fs";

const KEY = process.env.STRIPE_SECRET_KEY;
const COMP = process.env.COMP_CREDS || "";
const EDGE_DIR = "docs/edge";

if (!existsSync("build/early_payload.json")) { console.log("publish_edge: no early_payload.json; skipping"); process.exit(0); }
const payload = readFileSync("build/early_payload.json", "utf8");

function norm(s) { return (s || "").toUpperCase().replace(/[^A-Z0-9]+/g, "-").replace(/^-+|-+$/g, ""); }
function sha256hex(s) { return createHash("sha256").update(s).digest("hex"); }
function encrypt(plaintext, cred) {
  const salt = randomBytes(16), iv = randomBytes(12), key = pbkdf2Sync(cred, salt, 100000, 32, "sha256");
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

const creds = new Set();

// Comp "god passes": premium for free, for anyone you give the code to.
let compCount = 0;
for (const raw of COMP.split(/[\s,]+/)) {
  const c = norm(raw);
  if (c) { creds.add(c); compCount++; }
}

// Active paying subscribers (credential carried as client_reference_id).
let subCount = 0;
if (KEY) {
  try {
    const sessions = await stripe("checkout/sessions?limit=100&expand[]=data.subscription");
    for (const s of sessions.data || []) {
      const cred = norm(s.client_reference_id);
      const sub = s.subscription;
      if (cred && sub && typeof sub === "object" && ["active", "trialing"].includes(sub.status)) {
        if (!creds.has(cred)) subCount++;
        creds.add(cred);
      }
    }
  } catch (e) {
    console.log("publish_edge: stripe list failed (" + e.message + "); publishing comp passes only");
  }
}

mkdirSync(EDGE_DIR, { recursive: true });
const keep = new Set();
for (const cred of creds) {
  const name = sha256hex(cred) + ".json";
  keep.add(name);
  writeFileSync(EDGE_DIR + "/" + name, JSON.stringify(encrypt(payload, cred)));
}
for (const f of readdirSync(EDGE_DIR)) {
  if (f.endsWith(".json") && !keep.has(f)) rmSync(EDGE_DIR + "/" + f);
}
console.log("publish_edge: published " + keep.size + " feed(s) (" + subCount + " subscriber, " + compCount + " comp)");
