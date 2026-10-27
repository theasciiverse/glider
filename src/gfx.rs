//! A 512x342 picture and the part of QuickDraw that Glider draws with:
//! rectangles, frames, lines with a pen size, ovals and polygon regions in
//! the 50% gray pattern, masked and stretched copies from the art, inverts
//! and bitmap text. Coordinates are the original window's.

use crate::assets::Assets;
use crate::house::Rect;
use crate::mac::{Font, Image, Rgb};

pub const W: i32 = 512;
pub const H: i32 = 342;

pub const BLACK: Rgb = [0, 0, 0];
pub const WHITE: Rgb = [255, 255, 255];
pub const YELLOW: Rgb = [255, 255, 0];
pub const RED: Rgb = [255, 0, 0];
pub const LT_BLUE: Rgb = [0, 255, 255];
/// SetUpColors' RGB values, 16-bit, here as 8-bit.
pub const BROWN: Rgb = [86, 44, 5];
pub const LT_BROWN: Rgb = [144, 113, 58];
pub const DK_GRAY: Rgb = [64, 64, 64];
pub const LT_GREEN: Rgb = [31, 183, 20];

pub const WHOLE: Rect = Rect::new(0, 0, W, H);

#[derive(Clone)]
pub struct Canvas {
    pub px: Vec<Rgb>,
    /// Drawing is limited to this rectangle (SetClip with a rectangle).
    pub clip: Rect,
    /// When set, masked sprites are painted solid in this colour: the
    /// silhouette view.
    pub ink: Option<Rgb>,
    /// The silhouettes alone, pixel by pixel, when not empty: what the
    /// silhouette view draws in sharper characters than the room behind.
    /// The art's dark parts inside a shape (a glider's lower wing and its
    /// folds) are a darker shade of the ink there. The flag is set for
    /// silhouettes, whose thin parts must not vanish into the room, and
    /// clear for the clock, drawn as sharply (`draw::flat_clock`).
    pub fore: Vec<Option<(Rgb, bool)>>,
}

/// A silhouette's ink where the art is dark.
pub fn shade(c: Rgb) -> Rgb {
    c.map(|v| (v as u32 * 9 / 20) as u8)
}

/// The 50% gray pattern, $AA55...: set where x + y is even.
fn gray(x: i32, y: i32) -> bool {
    (x + y) & 1 == 0
}

impl Canvas {
    pub fn new() -> Canvas {
        Canvas { px: vec![WHITE; (W * H) as usize], clip: WHOLE, ink: None, fore: Vec::new() }
    }

    pub fn set(&mut self, x: i32, y: i32, c: Rgb) {
        let r = self.clip;
        if x >= r.left.max(0) && x < r.right.min(W) && y >= r.top.max(0) && y < r.bottom.min(H) {
            self.px[(y * W + x) as usize] = c;
        }
    }

    fn set_fore(&mut self, x: i32, y: i32, c: (Rgb, bool)) {
        let r = self.clip;
        if x >= r.left.max(0) && x < r.right.min(W) && y >= r.top.max(0) && y < r.bottom.min(H) {
            self.fore[(y * W + x) as usize] = Some(c);
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Rgb {
        if (0..W).contains(&x) && (0..H).contains(&y) { self.px[(y * W + x) as usize] } else { BLACK }
    }

    pub fn clip_to(&mut self, r: Rect) {
        self.clip = r;
    }

    pub fn unclip(&mut self) {
        self.clip = WHOLE;
    }

    /// PaintRect / FillRect / EraseRect.
    pub fn fill(&mut self, r: Rect, c: Rgb) {
        for y in r.top..r.bottom {
            for x in r.left..r.right {
                self.set(x, y, c);
            }
        }
    }

    /// FrameRect: a one-pixel border just inside `r`.
    pub fn frame(&mut self, r: Rect, c: Rgb) {
        if r.empty() {
            return;
        }
        for x in r.left..r.right {
            self.set(x, r.top, c);
            self.set(x, r.bottom - 1, c);
        }
        for y in r.top..r.bottom {
            self.set(r.left, y, c);
            self.set(r.right - 1, y, c);
        }
    }

    pub fn invert(&mut self, r: Rect) {
        for y in r.top.max(0)..r.bottom.min(H) {
            for x in r.left.max(0)..r.right.min(W) {
                let p = self.get(x, y);
                self.set(x, y, [255 - p[0], 255 - p[1], 255 - p[2]]);
            }
        }
    }

    /// MoveTo(x0, y0); LineTo(x1, y1) with a pen of `pw` x `ph`, which hangs
    /// below and to the right of the line.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, pw: i32, ph: i32, c: Rgb) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            for py in 0..ph {
                for px in 0..pw {
                    self.set(x + px, y + py, c);
                }
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// PaintOval in the gray pattern, transparent: black where the pattern
    /// is set, the picture untouched elsewhere.
    pub fn gray_oval(&mut self, r: Rect) {
        let (cx, cy) = ((r.left + r.right) as f32 / 2.0, (r.top + r.bottom) as f32 / 2.0);
        let (rx, ry) = (r.width() as f32 / 2.0, r.height() as f32 / 2.0);
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        for y in r.top..r.bottom {
            for x in r.left..r.right {
                let (nx, ny) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                if nx * nx + ny * ny <= 1.0 && gray(x, y) {
                    self.set(x, y, BLACK);
                }
            }
        }
    }

    /// PaintRgn of a region traced with lines (OpenRgn ... CloseRgn), in the
    /// gray pattern, transparent.
    pub fn gray_polygon(&mut self, pts: &[(i32, i32)]) {
        let (Some(y0), Some(y1)) = (pts.iter().map(|p| p.1).min(), pts.iter().map(|p| p.1).max()) else { return };
        let (x0, x1) = (pts.iter().map(|p| p.0).min().unwrap_or(0), pts.iter().map(|p| p.0).max().unwrap_or(0));
        for y in y0..y1 {
            for x in x0..x1 {
                if gray(x, y) && inside(pts, x as f32 + 0.5, y as f32 + 0.5) {
                    self.set(x, y, BLACK);
                }
            }
        }
    }

    /// CopyMask from the object sheet: `src` of the art drawn into `dst`
    /// wherever the mask at `msk` is black. Stretches if the sizes differ.
    pub fn copy_mask(&mut self, a: &Assets, src: Rect, msk: Rect, dst: Rect) {
        let (dw, dh) = (dst.width(), dst.height());
        if dw <= 0 || dh <= 0 || src.empty() {
            return;
        }
        for y in 0..dh {
            for x in 0..dw {
                let (sx, sy) = (x * src.width() / dw, y * src.height() / dh);
                if a.mask_at(msk.left + sx, msk.top + sy) {
                    let c = self.ink.unwrap_or_else(|| a.objects.at(src.left + sx, src.top + sy));
                    self.set(dst.left + x, dst.top + y, c);
                    if self.ink.is_some() && !self.fore.is_empty() {
                        let (ax, ay) = (msk.left + sx, msk.top + sy);
                        let inside = a.mask_at(ax - 1, ay) && a.mask_at(ax + 1, ay) && a.mask_at(ax, ay - 1) && a.mask_at(ax, ay + 1);
                        let dark = a.objects.at(src.left + sx, src.top + sy).iter().all(|&v| v < 100);
                        self.set_fore(dst.left + x, dst.top + y, (if inside && dark { shade(c) } else { c }, true));
                    }
                }
            }
        }
    }

    /// A thing of the house drawn from its art for the silhouette view's
    /// matcher (`draw::sharp_art`): without the black outline around it,
    /// a dark fringe at terminal sizes, and recorded in `fore` in its own
    /// colours.
    pub fn art(&mut self, a: &Assets, src: Rect, dst: Rect) {
        let (dw, dh) = (dst.width(), dst.height());
        if dw <= 0 || dh <= 0 || src.empty() {
            return;
        }
        for y in 0..dh {
            for x in 0..dw {
                let (ax, ay) = (src.left + x * src.width() / dw, src.top + y * src.height() / dh);
                if !a.mask_at(ax, ay) {
                    continue;
                }
                let c = a.objects.at(ax, ay);
                let edge = !(a.mask_at(ax - 1, ay) && a.mask_at(ax + 1, ay) && a.mask_at(ax, ay - 1) && a.mask_at(ax, ay + 1));
                if !(edge && c.iter().all(|&v| v < 64)) {
                    self.put(dst.left + x, dst.top + y, c);
                }
            }
        }
    }

    /// A pixel drawn and recorded in `fore` as a thing of the house, for a
    /// picture drawn for the silhouette view (GameOver's paper letters).
    pub fn put(&mut self, x: i32, y: i32, c: Rgb) {
        self.set(x, y, c);
        if !self.fore.is_empty() {
            self.set_fore(x, y, (c, false));
        }
    }

    /// Records in `fore` what changed in `r` since `before` (a copy of `px`),
    /// as a thing of the house: a picture drawn for the silhouette view, as
    /// the clock is.
    pub fn keep_changes(&mut self, before: &[Rgb], r: Rect) {
        if self.fore.is_empty() {
            return;
        }
        for y in r.top.max(0)..r.bottom.min(H) {
            for x in r.left.max(0)..r.right.min(W) {
                let k = (y * W + x) as usize;
                if self.px[k] != before[k] {
                    self.fore[k] = Some((self.px[k], false));
                }
            }
        }
    }

    /// The common case: a sprite through its own mask.
    pub fn sprite(&mut self, a: &Assets, src: Rect, dst: Rect) {
        self.copy_mask(a, src, src, dst);
    }

    /// CopyBits (srcCopy), stretching `src` of an image into `dst`.
    pub fn copy_bits(&mut self, img: &Image, src: Rect, dst: Rect) {
        let (dw, dh) = (dst.width(), dst.height());
        if dw <= 0 || dh <= 0 || src.empty() {
            return;
        }
        for y in 0..dh {
            for x in 0..dw {
                let (sx, sy) = (src.left + x * src.width() / dw, src.top + y * src.height() / dh);
                self.set(dst.left + x, dst.top + y, img.at(sx, sy));
            }
        }
    }

    /// Copy part of another canvas.
    pub fn copy_from(&mut self, o: &Canvas, r: Rect) {
        for y in r.top.max(0)..r.bottom.min(H) {
            for x in r.left.max(0)..r.right.min(W) {
                let c = o.px[(y * W + x) as usize];
                self.set(x, y, c);
            }
        }
    }

    /// DrawString with the baseline at (x, y), in srcOr: only the letters'
    /// pixels are drawn.
    pub fn text(&mut self, f: &Font, s: &str, x: i32, y: i32, c: Rgb) {
        let bytes: Vec<u8> = s.bytes().collect();
        f.draw(&bytes, x, y, |px, py| self.set(px, py, c));
    }
}

impl Canvas {
    /// Blur with a (2r+1)-square box and scale the brightness by `k`: what
    /// the silhouette view does to a room's background, so the wallpaper's
    /// dithering and fine lines become calm shades.
    pub fn soften(&mut self, r: i32, k: f32) {
        let (w, h) = (W as usize, H as usize);
        // summed-area table, one per channel
        let mut sat = vec![[0u32; 3]; (w + 1) * (h + 1)];
        for y in 0..h {
            let mut row = [0u32; 3];
            for x in 0..w {
                let p = self.px[y * w + x];
                for c in 0..3 {
                    row[c] += p[c] as u32;
                    sat[(y + 1) * (w + 1) + x + 1][c] = sat[y * (w + 1) + x + 1][c] + row[c];
                }
            }
        }
        for y in 0..H {
            for x in 0..W {
                let (x0, x1) = ((x - r).max(0) as usize, (x + r + 1).min(W) as usize);
                let (y0, y1) = ((y - r).max(0) as usize, (y + r + 1).min(H) as usize);
                let n = ((x1 - x0) * (y1 - y0)) as f32;
                let mut out = [0u8; 3];
                for c in 0..3 {
                    let s = sat[y1 * (w + 1) + x1][c] + sat[y0 * (w + 1) + x0][c] - sat[y0 * (w + 1) + x1][c] - sat[y1 * (w + 1) + x0][c];
                    out[c] = (s as f32 / n * k).min(255.0) as u8;
                }
                self.px[(y * W + x) as usize] = out;
            }
        }
    }
}

/// Even-odd point in polygon.
fn inside(pts: &[(i32, i32)], x: f32, y: f32) -> bool {
    let mut c = false;
    let n = pts.len();
    for i in 0..n {
        let (xi, yi) = (pts[i].0 as f32, pts[i].1 as f32);
        let (xj, yj) = (pts[(i + n - 1) % n].0 as f32, pts[(i + n - 1) % n].1 as f32);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            c = !c;
        }
    }
    c
}

/// A region outline traced the QuickDraw way: start at a point, then
/// relative moves (`Line`) or absolute ones (`LineTo`).
pub struct Trace {
    pub pts: Vec<(i32, i32)>,
}

impl Trace {
    pub fn from(x: i32, y: i32) -> Trace {
        Trace { pts: vec![(x, y)] }
    }

    fn last(&self) -> (i32, i32) {
        *self.pts.last().unwrap_or(&(0, 0))
    }

    pub fn by(mut self, dx: i32, dy: i32) -> Trace {
        let (x, y) = self.last();
        self.pts.push((x + dx, y + dy));
        self
    }

    pub fn to(mut self, x: i32, y: i32) -> Trace {
        self.pts.push((x, y));
        self
    }
}
