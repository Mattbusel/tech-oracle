use super::*;

#[test]
fn day_diff_counts_forward_and_clamps() {
    assert_eq!(day_diff("2026-06-01", "2026-06-10"), 9);
    assert_eq!(day_diff("2026-06-10", "2026-06-01"), 0);
}

#[test]
fn hsl_to_hex_primaries() {
    assert_eq!(hsl_hex(0.0, 1.0, 0.5), "#ff0000");
    assert_eq!(hsl_hex(1.0 / 3.0, 1.0, 0.5), "#00ff00");
}

#[test]
fn slug_and_xml_and_enc() {
    assert_eq!(slug("Hello, World!"), "hello-world");
    assert_eq!(xml("<a & b>"), "&lt;a &amp; b&gt;");
    assert_eq!(enc("a b"), "a%20b");
}

#[test]
fn clip_keeps_short_and_truncates_long() {
    assert_eq!(clip_r("short", 50), "short");
    let long = "a".repeat(100);
    let c = clip_r(&long, 10);
    assert_eq!(c.chars().count(), 10);
    assert!(c.ends_with("..."));
}

#[test]
fn wrap_breaks_on_words() {
    let lines = wrap_chars("one two three four five", 9);
    assert!(lines.len() >= 2);
    for l in &lines {
        assert!(l.len() <= 13);
    }
}
