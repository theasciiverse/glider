//! The 512x342 picture in the terminal, in half-block cells (`▀`): two
//! pixels a cell, each its own colour. The picture keeps the Mac's square
//! pixels, so its size comes from the terminal's cell shape; each terminal
//! pixel is the average of the Mac pixels it covers, which keeps the
//! backgrounds' dithering and the gray shadows as the shades they were
//! meant to be.

use crate::gfx::{Canvas, H, W};
use crate::mac::Rgb;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

/// Height over width of a terminal cell.
pub fn cell_aspect() -> f32 {
    match ratatui::crossterm::terminal::window_size() {
        Ok(w) if w.width > 0 && w.height > 0 && w.columns > 0 && w.rows > 0 => {
            let a = (w.height as f32 / w.rows as f32) / (w.width as f32 / w.columns as f32);
            if (1.0..4.0).contains(&a) { a } else { 2.0 }
        }
        _ => 2.0,
    }
}

/// Where the picture goes in `area`: the biggest rectangle of its shape,
/// centred, in cells, and its size in half-block pixels.
#[derive(Clone, Copy, Debug, Default)]
pub struct Placed {
    pub cells: Rect,
    pub tw: u16,
    pub th: u16,
}

pub fn place(area: Rect, aspect: f32) -> Placed {
    // a half-block pixel is aspect/2 times as tall as it is wide
    let dot_h = aspect / 2.0;
    let max_w = area.width as f32;
    let max_h = (area.height * 2) as f32;
    let ratio = W as f32 / H as f32;
    let tw = max_w.min(ratio * max_h * dot_h);
    let th = (tw / (ratio * dot_h)).min(max_h);
    let cols = (tw.round() as u16).clamp(1, area.width.max(1));
    let rows = ((th / 2.0).round() as u16).clamp(1, area.height.max(1));
    let cells = Rect::new(area.x + (area.width - cols) / 2, area.y + (area.height - rows) / 2, cols, rows);
    Placed { cells, tw: cols, th: rows * 2 }
}

impl Placed {
    /// The room's x under a terminal column, for mouse control.
    pub fn room_x(&self, col: u16) -> Option<i32> {
        if self.tw == 0 {
            return None;
        }
        let c = col as i32 - self.cells.x as i32;
        Some(((c * 2 + 1) * W / (self.tw as i32 * 2)).clamp(0, W - 1))
    }

    pub fn contains(&self, col: u16, row: u16) -> bool {
        self.cells.contains(ratatui::layout::Position { x: col, y: row })
    }
}

fn spans(n: u16, len: i32) -> Vec<(i32, i32)> {
    (0..n as i32)
        .map(|t| {
            let a = t * len / n as i32;
            let b = ((t + 1) * len / n as i32).max(a + 1).min(len);
            (a, b)
        })
        .collect()
}

fn rgb(c: Rgb) -> Color {
    Color::Rgb(c[0], c[1], c[2])
}

/// Draw the picture into `p`.
pub fn draw(buf: &mut Buffer, c: &Canvas, p: &Placed) {
    let xs = spans(p.tw, W);
    let ys = spans(p.th, H);
    let avg = |tx: usize, ty: usize| -> Rgb {
        let (x0, x1) = xs[tx];
        let (y0, y1) = ys[ty];
        let mut sum = [0u32; 3];
        for y in y0..y1 {
            for x in x0..x1 {
                let px = c.px[(y * W + x) as usize];
                for k in 0..3 {
                    sum[k] += px[k] as u32;
                }
            }
        }
        let n = ((x1 - x0) * (y1 - y0)).max(1) as u32;
        [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
    };
    let r = p.cells;
    for cy in 0..r.height {
        for cx in 0..r.width {
            let top = avg(cx as usize, cy as usize * 2);
            let bot = avg(cx as usize, cy as usize * 2 + 1);
            let cell = &mut buf[(r.x + cx, r.y + cy)];
            if top == bot {
                cell.set_char(' ').set_fg(Color::Rgb(0, 0, 0)).set_bg(rgb(top));
            } else {
                cell.set_char('▀').set_fg(rgb(top)).set_bg(rgb(bot));
            }
        }
    }
}

// ---------------------------------------------------------------------
// the silhouette view: Braille shapes and box-drawing edges

/// Where the picture goes for Braille: the same cells, 2x4 dots each.
pub fn dots(p: &Placed) -> (u16, u16) {
    (p.cells.width * 2, p.cells.height * 4)
}

fn lum(c: Rgb) -> u32 {
    c[0] as u32 * 2 + c[1] as u32 * 5 + c[2] as u32
}

fn dist(a: Rgb, b: Rgb) -> u32 {
    let d = |i: usize| (a[i] as i32 - b[i] as i32).pow(2) as u32;
    d(0) + d(1) + d(2)
}

/// Two colours this close are one colour as far as a cell can show.
const SAME: u32 = 3 * 36 * 36;

/// One cell's 2x4 patch (`p[x][y]`) in at most two colours, as AQuest's
/// Mixed view does it: the background is the colour covering most of the
/// cell, the dots the colour that stands out most from it, every other
/// sample joins the nearer of the two. A cell split top and bottom, or all
/// one colour, is a half block or a space instead, which shows solid areas
/// more solidly.
fn braille_cell(p: &[[Rgb; 4]; 2]) -> (char, Rgb, Rgb) {
    const BIT: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
    const TOP: u32 = 0x1B;
    let at = |i: usize| p[i / 4][i % 4];
    let mut groups: Vec<(Rgb, Rgb, u32)> = Vec::new();
    for i in 0..8 {
        let c = at(i);
        match groups.iter_mut().find(|g| dist(g.0, c) < SAME) {
            Some(g) => {
                g.2 += 1;
                if lum(c) > lum(g.1) {
                    g.1 = c;
                }
            }
            None => groups.push((c, c, 1)),
        }
    }
    let bg = *groups.iter().max_by_key(|g| (g.2, std::cmp::Reverse(lum(g.1)))).unwrap();
    let Some(fg) = groups.iter().filter(|g| g.0 != bg.0).max_by_key(|g| g.2 as u64 * dist(g.1, bg.1) as u64).copied() else {
        return (' ', bg.1, bg.1);
    };
    let mut bits = 0;
    for i in 0..8 {
        let c = at(i);
        if dist(c, fg.0) < dist(c, bg.0) {
            bits |= BIT[i / 4][i % 4];
        }
    }
    match bits {
        0 => (' ', bg.1, bg.1),
        0xFF => (' ', fg.1, fg.1),
        TOP => ('▀', fg.1, bg.1),
        b if b == 0xFF & !TOP => ('▀', bg.1, fg.1),
        b => (char::from_u32(0x2800 + b).unwrap_or(' '), fg.1, bg.1),
    }
}

/// The picture in Braille cells. Each dot is the brightest Mac pixel under
/// it, so thin bright things (a glider's wing, a spark) never vanish.
pub fn draw_braille(buf: &mut Buffer, c: &Canvas, p: &Placed) {
    let (tw, th) = dots(p);
    let xs = spans(tw, W);
    let ys = spans(th, H);
    let at = |tx: usize, ty: usize| -> Rgb {
        let (x0, x1) = xs[tx];
        let (y0, y1) = ys[ty];
        let mut best = c.px[(y0 * W + x0) as usize];
        for y in y0..y1 {
            for x in x0..x1 {
                let px = c.px[(y * W + x) as usize];
                if lum(px) > lum(best) {
                    best = px;
                }
            }
        }
        best
    };
    let r = p.cells;
    for cy in 0..r.height {
        for cx in 0..r.width {
            let mut patch = [[[0u8; 3]; 4]; 2];
            for (x, col) in patch.iter_mut().enumerate() {
                for (y, px) in col.iter_mut().enumerate() {
                    *px = at(cx as usize * 2 + x, cy as usize * 4 + y);
                }
            }
            let (ch, fg, bg) = braille_cell(&patch);
            buf[(r.x + cx, r.y + cy)].set_char(ch).set_fg(rgb(fg)).set_bg(rgb(bg));
        }
    }
}

/// The house's straight edges in box-drawing characters over the picture,
/// skipping the cells under anything that moves, so the glider passes in
/// front of a table, not behind its outline. Bars and posts are heavy;
/// where one ends in the middle of a cell it ends in a half line.
pub fn draw_edges(buf: &mut Buffer, p: &Placed, edges: &[(crate::draw::Edge, Rgb)], movers: &[crate::house::Rect]) {
    use crate::draw::Edge;
    let r = p.cells;
    let (cols, rows) = (r.width as f32, r.height as f32);
    let fx = |x: i32| x as f32 * cols / W as f32;
    let fy = |y: i32| y as f32 * rows / H as f32;
    let blocked: Vec<(i32, i32, i32, i32)> = movers
        .iter()
        .filter(|m| !m.empty())
        .map(|m| (fx(m.left).floor() as i32, fy(m.top).floor() as i32, fx(m.right).ceil() as i32, fy(m.bottom).ceil() as i32))
        .collect();
    let put = |buf: &mut Buffer, cx: i32, cy: i32, ch: char, col: Rgb| {
        if cx < 0 || cy < 0 || cx >= r.width as i32 || cy >= r.height as i32 {
            return;
        }
        if blocked.iter().any(|&(x0, y0, x1, y1)| cx >= x0 && cx < x1 && cy >= y0 && cy < y1) {
            return;
        }
        let cell = &mut buf[(r.x + cx as u16, r.y + cy as u16)];
        // a bar landing on a post's top makes a tee
        let ch = match (cell.symbol(), ch) {
            ("┃" | "╻", '━') => '┳',
            _ => ch,
        };
        // keep the cell's background, the edge in front of it
        let bg = cell.bg;
        let bg = if bg == ratatui::style::Color::Reset { rgb([0; 3]) } else { bg };
        cell.set_char(ch).set_fg(rgb(col)).set_bg(bg);
    };
    for &(e, col) in edges {
        match e {
            Edge::Post(b) => {
                let cx = fx((b.left + b.right) / 2).floor() as i32;
                let (y0, y1) = (fy(b.top), fy(b.bottom));
                let (top, bot) = (y0.floor() as i32, (y1.ceil() as i32 - 1).max(y0.floor() as i32));
                for cy in top..=bot {
                    let ch = if cy == top && y0.fract() >= 0.5 {
                        '╻'
                    } else if cy == bot && y1 - bot as f32 <= 0.5 {
                        '╹'
                    } else {
                        '┃'
                    };
                    put(buf, cx, cy, ch, col);
                }
            }
            Edge::Bar(b) => {
                let cy = fy((b.top + b.bottom) / 2).floor() as i32;
                let (x0, x1) = (fx(b.left), fx(b.right));
                let (first, last) = (x0.floor() as i32, (x1.ceil() as i32 - 1).max(x0.floor() as i32));
                for cx in first..=last {
                    let ch = if cx == first && x0.fract() >= 0.5 {
                        '╺'
                    } else if cx == last && x1 - last as f32 <= 0.5 {
                        '╸'
                    } else {
                        '━'
                    };
                    put(buf, cx, cy, ch, col);
                }
            }
            Edge::Outline(b) => {
                let (left, right) = (fx(b.left).floor() as i32, (fx(b.right).ceil() as i32 - 1));
                let (top, bot) = (fy(b.top).floor() as i32, (fy(b.bottom).ceil() as i32 - 1));
                if right - left < 2 || bot - top < 1 {
                    continue;
                }
                for cx in left + 1..right {
                    put(buf, cx, top, '─', col);
                    put(buf, cx, bot, '─', col);
                }
                for cy in top + 1..bot {
                    put(buf, left, cy, '│', col);
                    put(buf, right, cy, '│', col);
                }
                put(buf, left, top, '┌', col);
                put(buf, right, top, '┐', col);
                put(buf, left, bot, '└', col);
                put(buf, right, bot, '┘', col);
            }
        }
    }
}
