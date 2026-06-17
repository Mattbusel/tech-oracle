//! Access codes: encrypt the day's early feed once per code so that a shared
//! code (a "god pass" or a subscriber code) unlocks it in the browser. The
//! format matches the client: PBKDF2-HMAC-SHA256(100k) -> AES-256-GCM, the
//! ciphertext is `ct||tag`, all fields base64. Written to docs/edge/<hash>.json
//! where hash = sha256hex(normalized code). Public files, useless without a code.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};

fn norm(code: &str) -> String {
    let up = code.to_uppercase();
    let mut out = String::with_capacity(up.len());
    let mut prev_dash = false;
    for ch in up.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn sha256hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn encrypt(plaintext: &str, code: &str) -> anyhow::Result<String> {
    let mut salt = [0u8; 16];
    let mut iv = [0u8; 12];
    getrandom::getrandom(&mut salt).map_err(|e| anyhow::anyhow!("rng: {e}"))?;
    getrandom::getrandom(&mut iv).map_err(|e| anyhow::anyhow!("rng: {e}"))?;

    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(code.as_bytes(), &salt, 100_000, &mut key);

    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow::anyhow!("key: {e}"))?;
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("encrypt: {e}"))?; // returns ciphertext||tag

    Ok(serde_json::json!({
        "v": 1,
        "salt": STANDARD.encode(salt),
        "iv": STANDARD.encode(iv),
        "ct": STANDARD.encode(ct),
    })
    .to_string())
}

/// Publish an encrypted copy of `payload_json` for every code in `codes`
/// (comma/space/newline separated), and remove edge files for codes no longer
/// listed (revocation). No-op when `codes` is empty.
pub fn publish(edge_dir: &str, payload_json: &str, codes: &str) {
    // Codes are separated by commas or newlines; a single code may contain
    // spaces (normalized to hyphens), so "GOLDEN TICKET" is one code.
    let list: Vec<String> = codes
        .split([',', '\n', '\r'])
        .map(norm)
        .filter(|c| !c.is_empty())
        .collect();

    let _ = std::fs::create_dir_all(edge_dir);

    let mut keep = std::collections::HashSet::new();
    for code in &list {
        let name = format!("{}.json", sha256hex(code));
        match encrypt(payload_json, code) {
            Ok(blob) => {
                if std::fs::write(format!("{edge_dir}/{name}"), blob).is_ok() {
                    keep.insert(name);
                }
            }
            Err(e) => eprintln!("access: encrypt failed ({e})"),
        }
    }

    // Revoke: drop any edge file whose code is no longer listed.
    if let Ok(entries) = std::fs::read_dir(edge_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.ends_with(".json") && !keep.contains(&fname) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    eprintln!("access: published {} code feed(s)", keep.len());
}

#[cfg(test)]
#[path = "tests_access.rs"]
mod tests_access;
