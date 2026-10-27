//! The classic Mac formats Glider 4.0 ships in, read directly so the game
//! uses John Calhoun's own files: MacBinary (the art files), resource
//! forks, DeRez text (`Glider.r`, `snd.r`), QuickDraw pictures (PICT 1 and
//! 2), sampled sounds (`snd `) and bitmap fonts (`FONT`).

use std::collections::HashMap;

pub type Rgb = [u8; 3];

/// Resources by (type, id).
pub type Resources = HashMap<([u8; 4], i16), Vec<u8>>;

fn be16(d: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([d[at], d[at + 1]])
}

fn be32(d: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

/// The resource fork of a MacBinary file.
pub fn macbinary_rsrc(d: &[u8]) -> &[u8] {
    let dlen = be32(d, 83) as usize;
    let rlen = be32(d, 87) as usize;
    let at = 128 + dlen.div_ceil(128) * 128;
    &d[at..at + rlen]
}

/// Every resource in a resource fork.
pub fn resource_fork(rf: &[u8]) -> Resources {
    let mut out = HashMap::new();
    let data_off = be32(rf, 0) as usize;
    let map_off = be32(rf, 4) as usize;
    let m = &rf[map_off..];
    let tl = be16(m, 24) as usize;
    let types = be16(m, tl) as usize + 1;
    for t in 0..types {
        let e = tl + 2 + t * 8;
        let kind = [m[e], m[e + 1], m[e + 2], m[e + 3]];
        let count = be16(m, e + 4) as usize + 1;
        let refs = tl + be16(m, e + 6) as usize;
        for r in 0..count {
            let re = refs + r * 12;
            let id = be16(m, re) as i16;
            let at = data_off + (be32(m, re + 4) & 0xFF_FFFF) as usize;
            let len = be32(rf, at) as usize;
            out.insert((kind, id), rf[at + 4..at + 4 + len].to_vec());
        }
    }
    out
}

/// Every resource in DeRez text: `data 'TYPE' (id, ...) {` then lines of
/// `$"0A1B ..."`, ending `};`. Line ends may be CR or LF.
pub fn derez(text: &[u8]) -> Resources {
    let mut out = HashMap::new();
    let mut cur: Option<(([u8; 4], i16), Vec<u8>)> = None;
    for line in text.split(|&b| b == b'\r' || b == b'\n') {
        if line.starts_with(b"data '") && line.len() > 12 {
            let kind = [line[6], line[7], line[8], line[9]];
            let rest = &line[11..];
            let open = rest.iter().position(|&b| b == b'(').map_or(0, |p| p + 1);
            let num: String = rest[open..].iter().take_while(|&&b| b == b'-' || b.is_ascii_digit()).map(|&b| b as char).collect();
            if let Ok(id) = num.parse::<i16>() {
                cur = Some(((kind, id), Vec::new()));
            }
        } else if line.starts_with(b"};") {
            if let Some((k, v)) = cur.take() {
                out.insert(k, v);
            }
        } else if let Some((_, v)) = cur.as_mut() {
            let Some(s) = line.windows(2).position(|w| w == b"$\"") else { continue };
            let hex = &line[s + 2..];
            let end = hex.iter().position(|&b| b == b'"').unwrap_or(hex.len());
            let digits: Vec<u8> = hex[..end].iter().filter(|b| b.is_ascii_hexdigit()).copied().collect();
            for pair in digits.chunks(2) {
                if pair.len() == 2 {
                    let h = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
                    v.push(h(pair[0]) << 4 | h(pair[1]));
                }
            }
        }
    }
    out
}

/// A decoded picture, white where nothing was drawn.
#[derive(Clone)]
pub struct Image {
    pub w: i32,
    pub h: i32,
    pub px: Vec<Rgb>,
}

impl Image {
    pub fn new(w: i32, h: i32, c: Rgb) -> Image {
        Image { w, h, px: vec![c; (w * h) as usize] }
    }

    pub fn at(&self, x: i32, y: i32) -> Rgb {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return [255; 3];
        }
        self.px[(y * self.w + x) as usize]
    }
}

fn unpack_bits(src: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < src.len() {
        let n = src[i] as usize;
        i += 1;
        if n < 128 {
            let end = (i + n + 1).min(src.len());
            out.extend_from_slice(&src[i..end]);
            i = end;
        } else if n > 128 {
            if let Some(&b) = src.get(i) {
                out.extend(std::iter::repeat_n(b, 257 - n));
            }
            i += 1;
        }
    }
}

/// A QuickDraw picture, version 1 or 2, as far as Glider's art goes: bit
/// and pixel maps (packed or not), clip regions and comments. Anything
/// else is an error.
pub fn pict(v: &[u8]) -> Result<Image, String> {
    let top = be16(v, 2) as i16 as i32;
    let left = be16(v, 4) as i16 as i32;
    let bottom = be16(v, 6) as i16 as i32;
    let right = be16(v, 8) as i16 as i32;
    let mut img = Image::new(right - left, bottom - top, [255; 3]);
    let mut p = 10;
    let v1 = v[p..p + 2] == [0x11, 0x01];
    if v1 {
        p += 2;
    } else if v[p..p + 4] == [0x00, 0x11, 0x02, 0xFF] {
        p += 4;
    } else {
        return Err("not a PICT".into());
    }
    let short = |p: usize| be16(v, p) as i16 as i32;
    while p < v.len() {
        if !v1 && p & 1 == 1 {
            p += 1;
        }
        let op = if v1 {
            p += 1;
            v[p - 1] as u16
        } else {
            p += 2;
            be16(v, p - 2)
        };
        match op {
            0x0000 | 0x001E => {}
            0x0C00 => p += 24,
            0x0001 => p += be16(v, p) as usize,
            0x00A0 => p += 2,
            0x00A1 => p += 4 + be16(v, p + 2) as usize,
            0x00FF => break,
            0x0090 | 0x0098 | 0x0099 => {
                let rb_raw = be16(v, p);
                p += 2;
                let pixmap = rb_raw & 0x8000 != 0;
                let row_bytes = (rb_raw & 0x3FFF) as usize;
                let (bt, bl, bb, br) = (short(p), short(p + 2), short(p + 4), short(p + 6));
                p += 8;
                let mut pal: HashMap<u16, Rgb> = HashMap::new();
                let depth;
                if pixmap {
                    p += 2 + 2 + 4 + 4 + 4; // version, packType, packSize, hRes, vRes
                    depth = be16(v, p + 2) as usize; // after pixelType
                    p += 8 + 12; // pixelType..cmpSize, planeBytes, pmTable, pmReserved
                    p += 4; // ctSeed
                    let flags = be16(v, p);
                    let n = be16(v, p + 2) as usize + 1;
                    p += 4;
                    for i in 0..n {
                        let idx = be16(v, p);
                        let c = [v[p + 2], v[p + 4], v[p + 6]];
                        pal.insert(if flags & 0x8000 != 0 { i as u16 } else { idx }, c);
                        p += 8;
                    }
                } else {
                    depth = 1;
                    pal.insert(0, [255; 3]);
                    pal.insert(1, [0; 3]);
                }
                let (st, sl, sb, sr) = (short(p), short(p + 2), short(p + 4), short(p + 6));
                let (dt, dl, db, dr) = (short(p + 8), short(p + 10), short(p + 12), short(p + 14));
                p += 18; // src, dst, mode
                if op == 0x0099 {
                    p += be16(v, p) as usize; // mask region, not needed
                }
                let mut line = Vec::with_capacity(row_bytes);
                let rows = (bb - bt) as usize;
                for y in 0..rows {
                    line.clear();
                    if op == 0x0090 || row_bytes < 8 {
                        line.extend_from_slice(&v[p..p + row_bytes]);
                        p += row_bytes;
                    } else {
                        let n = if row_bytes > 250 {
                            p += 2;
                            be16(v, p - 2) as usize
                        } else {
                            p += 1;
                            v[p - 1] as usize
                        };
                        unpack_bits(&v[p..p + n], &mut line);
                        p += n;
                    }
                    let sy = bt + y as i32;
                    if sy < st || sy >= sb {
                        continue;
                    }
                    let per_byte = 8 / depth;
                    let mask = (1u16 << depth) - 1;
                    for x in 0..(br - bl) {
                        let sx = bl + x;
                        if sx < sl || sx >= sr {
                            continue;
                        }
                        let Some(&byte) = line.get(x as usize / per_byte) else { continue };
                        let shift = 8 - depth * (x as usize % per_byte + 1);
                        let idx = (byte as u16 >> shift) & mask;
                        // source rectangle to destination, scaled as DrawPicture
                        // would only when they differ, which Glider's never do
                        let (ox, oy) = (dl + (sx - sl) * (dr - dl) / (sr - sl), dt + (sy - st) * (db - dt) / (sb - st));
                        let (ix, iy) = (ox - left, oy - top);
                        if ix >= 0 && iy >= 0 && ix < img.w && iy < img.h {
                            img.px[(iy * img.w + ix) as usize] = *pal.get(&idx).unwrap_or(&[255, 0, 255]);
                        }
                    }
                }
            }
            _ => return Err(format!("PICT opcode {op:#06x} at {p}")),
        }
    }
    Ok(img)
}

/// A sampled sound, 8-bit unsigned mono.
#[derive(Clone)]
pub struct Sound {
    pub rate: u32,
    pub data: Vec<u8>,
}

/// An `snd ` resource of format 1 or 2 holding one standard sound header.
pub fn snd(v: &[u8]) -> Option<Sound> {
    let mut p = 0;
    let format = be16(v, 0);
    p += 2;
    if format == 1 {
        let mods = be16(v, p) as usize;
        p += 2 + mods * 6;
    } else {
        p += 2; // refCount
    }
    let cmds = be16(v, p) as usize;
    p += 2;
    for c in 0..cmds {
        let at = p + c * 8;
        let cmd = be16(v, at) & 0x7FFF;
        if cmd == 0x51 || cmd == 0x50 {
            let h = be32(v, at + 4) as usize;
            let len = be32(v, h + 4) as usize;
            let rate = be32(v, h + 8) >> 16;
            let encode = v[h + 20];
            if encode != 0 {
                return None;
            }
            let start = h + 22;
            let data = v.get(start..start + len)?.to_vec();
            return Some(Sound { rate, data });
        }
    }
    None
}

/// A bitmap font (`FONT`).
pub struct Font {
    first: u16,
    last: u16,
    kern_max: i32,
    pub ascent: i32,
    pub height: i32,
    row_bytes: usize,
    image: Vec<u8>,
    loc: Vec<u16>,
    ow: Vec<u16>,
}

impl Font {
    pub fn parse(v: &[u8]) -> Option<Font> {
        let first = be16(v, 2);
        let last = be16(v, 4);
        let kern_max = be16(v, 8) as i16 as i32;
        let height = be16(v, 14) as i32;
        let ascent = be16(v, 18) as i32;
        let row_words = be16(v, 24) as usize;
        let row_bytes = row_words * 2;
        let img_at = 26;
        let img_len = row_bytes * height as usize;
        let n = (last - first) as usize + 3;
        let loc_at = img_at + img_len;
        let ow_at = loc_at + n * 2;
        if v.len() < ow_at + n * 2 {
            return None;
        }
        let loc = (0..n).map(|i| be16(v, loc_at + i * 2)).collect();
        let ow = (0..n).map(|i| be16(v, ow_at + i * 2)).collect();
        Some(Font { first, last, kern_max, ascent, height, row_bytes, image: v[img_at..img_at + img_len].to_vec(), loc, ow })
    }

    fn index(&self, c: u8) -> usize {
        let c = c as u16;
        let missing = (self.last - self.first + 1) as usize;
        if c < self.first || c > self.last || self.ow[(c - self.first) as usize] == 0xFFFF {
            missing
        } else {
            (c - self.first) as usize
        }
    }

    /// Pen advance for a character.
    pub fn advance(&self, c: u8) -> i32 {
        (self.ow[self.index(c)] & 0xFF) as i32
    }

    pub fn width(&self, s: &[u8]) -> i32 {
        s.iter().map(|&c| self.advance(c)).sum()
    }

    /// Calls `put(x, y)` for every set pixel of the string drawn with its
    /// baseline at `(x, y)`, as QuickDraw's DrawString does.
    pub fn draw(&self, s: &[u8], mut x: i32, y: i32, mut put: impl FnMut(i32, i32)) {
        for &c in s {
            let i = self.index(c);
            let off = (self.ow[i] >> 8) as i32;
            let (l0, l1) = (self.loc[i] as usize, self.loc[i + 1] as usize);
            let gx = x + self.kern_max + off;
            for row in 0..self.height as usize {
                for col in l0..l1 {
                    if self.image[row * self.row_bytes + col / 8] & (0x80 >> (col % 8)) != 0 {
                        put(gx + (col - l0) as i32, y - self.ascent + row as i32);
                    }
                }
            }
            x += self.advance(c);
        }
    }
}

/// A Pascal string from a byte slice whose first byte is the length.
pub fn pstring(b: &[u8]) -> Vec<u8> {
    let n = (b[0] as usize).min(b.len() - 1);
    b[1..1 + n].to_vec()
}

/// MacRoman to UTF-8, for names shown in the terminal.
pub fn mac_roman(b: &[u8]) -> String {
    const HIGH: &str = "ÄÅÇÉÑÖÜáàâäãåçéèêëíìîïñóòôöõúùûü†°¢£§•¶ß®©™´¨≠ÆØ∞±≤≥¥µ∂∑∏π∫ªºΩæø¿¡¬√ƒ≈∆«»… ÀÃÕŒœ\u{2013}\u{2014}“”‘’÷◊ÿŸ⁄€‹›ﬁﬂ‡·‚„‰ÂÊÁËÈÍÎÏÌÓÔ\u{F8FF}ÒÚÛÙıˆ˜¯˘˙˚¸˝˛ˇ";
    let high: Vec<char> = HIGH.chars().collect();
    b.iter().map(|&c| if c < 128 { c as char } else { high.get(c as usize - 128).copied().unwrap_or('?') }).collect()
}
