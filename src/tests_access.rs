use super::*;

#[test]
fn norm_canonicalizes_codes() {
    assert_eq!(norm("Golden Ticket!"), "GOLDEN-TICKET");
    assert_eq!(norm("  ribbon--copper  "), "RIBBON-COPPER");
    assert_eq!(norm("GOLDEN TICKET"), norm("golden-ticket"));
}

#[test]
fn sha256hex_is_stable() {
    assert_eq!(sha256hex("abc"), sha256hex("abc"));
    assert_eq!(sha256hex("abc").len(), 64);
    assert_ne!(sha256hex("abc"), sha256hex("abd"));
}

#[test]
fn encrypt_emits_parseable_envelope() {
    let blob = encrypt("{\"secret\":true}", "RIBBON-COPPER").unwrap();
    let v: serde_json::Value = serde_json::from_str(&blob).unwrap();
    assert_eq!(v["v"], 1);
    for k in ["salt", "iv", "ct"] {
        let b64 = v[k].as_str().unwrap();
        assert!(STANDARD.decode(b64).is_ok());
    }
}
