//! Server-side share-card rendering. Draws the dot-matrix card to an RGB buffer
//! (real impact dots, a hand-coded 5x7 font, no font files or services) and
//! encodes a PNG. These become the og:image for the site and every call page,
//! so every shared link unfurls as a branded visual.

use std::io::BufWriter;

type Rgb = (u8, u8, u8);
const PAPER: Rgb = (239, 237, 228);
const INK: Rgb = (27, 26, 20);
const SOFT: Rgb = (109, 107, 94);
const DESK: Rgb = (23, 24, 28);
const STAMP: Rgb = (178, 58, 46);
const GREEN: Rgb = (47, 111, 79);

struct Canvas {
    w: i32,
    h: i32,
    buf: Vec<u8>,
}

impl Canvas {
    fn new(w: i32, h: i32, bg: Rgb) -> Self {
        let mut buf = vec![0u8; (w * h * 3) as usize];
        for px in buf.chunks_exact_mut(3) {
            px[0] = bg.0;
            px[1] = bg.1;
            px[2] = bg.2;
        }
        Canvas { w, h, buf }
    }
    fn px(&mut self, x: i32, y: i32, c: Rgb) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = ((y * self.w + x) * 3) as usize;
        self.buf[i] = c.0;
        self.buf[i + 1] = c.1;
        self.buf[i + 2] = c.2;
    }
    fn dot(&mut self, cx: i32, cy: i32, r: f32, c: Rgb) {
        let ri = r.ceil() as i32;
        let r2 = r * r;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                if (dx * dx + dy * dy) as f32 <= r2 {
                    self.px(cx + dx, cy + dy, c);
                }
            }
        }
    }
    /// Draw a string as dot-matrix glyphs; returns the x advance end.
    fn text(&mut self, x: i32, y: i32, s: &str, pitch: i32, c: Rgb) -> i32 {
        let r = (pitch as f32 * 0.46).max(1.0);
        let mut cx = x;
        for ch in s.to_uppercase().chars() {
            let g = glyph(ch);
            for (col, byte) in g.iter().enumerate() {
                for row in 0..7 {
                    if byte & (1 << row) != 0 {
                        self.dot(cx + col as i32 * pitch, y + row as i32 * pitch, r, c);
                    }
                }
            }
            cx += 6 * pitch;
        }
        cx
    }
    fn save(&self, path: &str) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut enc = png::Encoder::new(BufWriter::new(file), self.w as u32, self.h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header()?;
        writer.write_image_data(&self.buf)?;
        Ok(())
    }
}

fn text_width(s: &str, pitch: i32) -> i32 {
    s.chars().count() as i32 * 6 * pitch
}

fn wrap(s: &str, pitch: i32, max: i32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let trial = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if text_width(&trial, pitch) > max && !cur.is_empty() {
            lines.push(cur);
            cur = word.to_string();
        } else {
            cur = trial;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn sprockets(c: &mut Canvas) {
    let mut x = 22;
    while x < c.w {
        c.dot(x, 17, 5.0, DESK);
        c.dot(x, c.h - 17, 5.0, DESK);
        x += 34;
    }
}

fn wordmark(c: &mut Canvas) {
    c.text(56, 44, "THE SIGNAL", 6, INK);
}

fn footer(c: &mut Canvas, site: &str) {
    let s = site.replace("https://", "").replace("http://", "");
    c.text(56, c.h - 58, &s, 3, SOFT);
}

fn stamp(c: &mut Canvas, text: &str, col: Rgb) {
    let pitch = 4;
    let w = text_width(text, pitch) + 24;
    let x = c.w - 56 - w;
    let y = 120;
    // box
    for t in 0..4 {
        for xx in x - t..x + w + t {
            c.px(xx, y - 12 - t, col);
            c.px(xx, y + 40 + t, col);
        }
        for yy in y - 12 - t..y + 40 + t {
            c.px(x - t, yy, col);
            c.px(x + w + t, yy, col);
        }
    }
    c.text(x + 12, y, text, pitch, col);
}

/// The homepage / daily og:image: the index, the record, and today's call.
pub fn site_card(path: &str, site: &str, date: &str, idx: i64, verdict: &str, hits: usize, misses: usize, call: &str) -> std::io::Result<()> {
    let mut c = Canvas::new(1200, 630, PAPER);
    sprockets(&mut c);
    wordmark(&mut c);
    c.text(56, 138, &format!("PUBLIC RECORD // {date}"), 3, SOFT);
    c.text(56, 196, &format!("INDEX {idx} {verdict}"), 6, INK);
    c.text(56, 286, &format!("RECORD {hits}-{misses}"), 6, INK);
    let mut y = 392;
    for line in wrap(call, 3, 1090).iter().take(4) {
        c.text(56, y, line, 3, SOFT);
        y += 34;
    }
    footer(&mut c, site);
    c.save(path)
}

/// Per-call og:image: the call, its market, and its verdict stamp.
pub fn call_card(path: &str, site: &str, no: i64, status: &str, market: &str, call: &str) -> std::io::Result<()> {
    let mut c = Canvas::new(1200, 630, PAPER);
    sprockets(&mut c);
    wordmark(&mut c);
    c.text(56, 138, &format!("CALL No. {no} // {market}"), 3, SOFT);
    let mut y = 210;
    for line in wrap(call, 5, 1080).iter().take(4) {
        c.text(56, y, line, 5, INK);
        y += 64;
    }
    let col = match status {
        "HIT" => GREEN,
        "MISS" => STAMP,
        _ => INK,
    };
    stamp(&mut c, status, col);
    footer(&mut c, site);
    c.save(path)
}

/// Render text as a 7-row ASCII dot-matrix banner (for the curl-able terminal
/// printout). Uses the same hand font as the PNG cards.
pub fn ascii_banner(text: &str) -> String {
    let chars: Vec<[u8; 5]> = text.to_uppercase().chars().map(glyph).collect();
    let mut out = String::new();
    for row in 0..7 {
        for g in &chars {
            for col in 0..5 {
                out.push(if g[col] & (1 << row) != 0 { '#' } else { ' ' });
            }
            out.push(' ');
        }
        while out.ends_with(' ') {
            out.pop();
        }
        out.push('\n');
    }
    out
}

/// 5x7 dot-matrix font. Each glyph is 5 columns; bit i (from 0) is row i (top).
fn glyph(ch: char) -> [u8; 5] {
    match ch {
        '0' => [0x3E, 0x51, 0x49, 0x45, 0x3E],
        '1' => [0x00, 0x42, 0x7F, 0x40, 0x00],
        '2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => [0x21, 0x41, 0x45, 0x4B, 0x31],
        '4' => [0x18, 0x14, 0x12, 0x7F, 0x10],
        '5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => [0x3C, 0x4A, 0x49, 0x49, 0x30],
        '7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => [0x06, 0x49, 0x49, 0x29, 0x1E],
        'A' => [0x7E, 0x11, 0x11, 0x11, 0x7E],
        'B' => [0x7F, 0x49, 0x49, 0x49, 0x36],
        'C' => [0x3E, 0x41, 0x41, 0x41, 0x22],
        'D' => [0x7F, 0x41, 0x41, 0x22, 0x1C],
        'E' => [0x7F, 0x49, 0x49, 0x49, 0x41],
        'F' => [0x7F, 0x09, 0x09, 0x09, 0x01],
        'G' => [0x3E, 0x41, 0x49, 0x49, 0x7A],
        'H' => [0x7F, 0x08, 0x08, 0x08, 0x7F],
        'I' => [0x00, 0x41, 0x7F, 0x41, 0x00],
        'J' => [0x20, 0x40, 0x41, 0x3F, 0x01],
        'K' => [0x7F, 0x08, 0x14, 0x22, 0x41],
        'L' => [0x7F, 0x40, 0x40, 0x40, 0x40],
        'M' => [0x7F, 0x02, 0x0C, 0x02, 0x7F],
        'N' => [0x7F, 0x04, 0x08, 0x10, 0x7F],
        'O' => [0x3E, 0x41, 0x41, 0x41, 0x3E],
        'P' => [0x7F, 0x09, 0x09, 0x09, 0x06],
        'Q' => [0x3E, 0x41, 0x51, 0x21, 0x5E],
        'R' => [0x7F, 0x09, 0x19, 0x29, 0x46],
        'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        'T' => [0x01, 0x01, 0x7F, 0x01, 0x01],
        'U' => [0x3F, 0x40, 0x40, 0x40, 0x3F],
        'V' => [0x1F, 0x20, 0x40, 0x20, 0x1F],
        'W' => [0x7F, 0x20, 0x18, 0x20, 0x7F],
        'X' => [0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' => [0x03, 0x04, 0x78, 0x04, 0x03],
        'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        '-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        '.' => [0x00, 0x60, 0x60, 0x00, 0x00],
        ',' => [0x00, 0x50, 0x30, 0x00, 0x00],
        '/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        ':' => [0x00, 0x36, 0x36, 0x00, 0x00],
        '!' => [0x00, 0x00, 0x5F, 0x00, 0x00],
        '%' => [0x23, 0x13, 0x08, 0x64, 0x62],
        '+' => [0x08, 0x08, 0x3E, 0x08, 0x08],
        '(' => [0x00, 0x1C, 0x22, 0x41, 0x00],
        ')' => [0x00, 0x41, 0x22, 0x1C, 0x00],
        '\'' => [0x00, 0x05, 0x03, 0x00, 0x00],
        '"' => [0x00, 0x07, 0x00, 0x07, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00],
    }
}

#[cfg(test)]
#[path = "tests_card.rs"]
mod tests_card;
