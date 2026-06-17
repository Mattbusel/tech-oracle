use super::*;

#[test]
fn ascii_banner_is_seven_rows() {
    let b = ascii_banner("AB");
    assert_eq!(b.lines().count(), 7);
}

#[test]
fn font_covers_alphanumerics() {
    for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars() {
        assert_ne!(glyph(ch), [0u8; 5], "missing glyph for {ch}");
    }
    // unknown chars render blank, never panic
    assert_eq!(glyph('\u{2603}'), [0u8; 5]);
}
