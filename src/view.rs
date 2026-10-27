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
            let (x0, x1) = (xs[cx as usize * 2].0, xs[cx as usize * 2 + 1].1);
            let (y0, y1) = (ys[cy as usize * 4].0, ys[cy as usize * 4 + 3].1);
            let (ch, fg, bg) = sharp_cell(c, x0, x1, y0, y1).unwrap_or_else(|| braille_cell(&patch));
            buf[(r.x + cx, r.y + cy)].set_char(ch).set_fg(rgb(fg)).set_bg(rgb(bg));
        }
    }
}

// ---------------------------------------------------------------------
// sharper silhouettes: a cell with something moving in it gets the
// character whose drawn shape comes nearest the sprite's there

/// A cell as 8x16 points, bit y * 8 + x.
type Shape = u128;
const GW: usize = 8;
const GH: usize = 16;

fn shape(f: impl Fn(usize, usize) -> bool) -> Shape {
    let mut s = 0;
    for y in 0..GH {
        for x in 0..GW {
            if f(x, y) {
                s |= 1 << (y * GW + x);
            }
        }
    }
    s
}

/// The characters to choose from besides Braille, each with the shape a
/// terminal draws it in. Block elements, box drawing and Braille are drawn
/// by the terminal itself in most (not from the font), so the shapes hold.
fn glyphs() -> &'static [(char, Shape)] {
    static G: std::sync::OnceLock<Vec<(char, Shape)>> = std::sync::OnceLock::new();
    G.get_or_init(|| {
        let mut g = vec![(' ', 0), ('█', shape(|_, _| true)), ('▀', shape(|_, y| y < GH / 2)), ('▔', shape(|_, y| y < 2))];
        for n in 1..8 {
            g.push((char::from_u32(0x2580 + n as u32).unwrap(), shape(|_, y| y >= GH - 2 * n)));
            g.push((char::from_u32(0x2590 - n as u32).unwrap(), shape(|x, _| x < n)));
        }
        g.push(('▐', shape(|x, _| x >= GW / 2)));
        g.push(('▕', shape(|x, _| x == GW - 1)));
        for (ch, q) in [('▘', 1), ('▝', 2), ('▖', 4), ('▗', 8), ('▚', 9), ('▞', 6), ('▙', 13), ('▛', 7), ('▜', 11), ('▟', 14)] {
            g.push((ch, shape(|x, y| q >> ((x >= GW / 2) as u32 + 2 * (y >= GH / 2) as u32) & 1 == 1)));
        }
        // lines: (left, right, up, down), 1 light, 2 heavy
        for (ch, arms) in [
            ('─', [1, 1, 0, 0]),
            ('│', [0, 0, 1, 1]),
            ('┌', [0, 1, 0, 1]),
            ('┐', [1, 0, 0, 1]),
            ('└', [0, 1, 1, 0]),
            ('┘', [1, 0, 1, 0]),
            ('╴', [1, 0, 0, 0]),
            ('╶', [0, 1, 0, 0]),
            ('╵', [0, 0, 1, 0]),
            ('╷', [0, 0, 0, 1]),
            ('━', [2, 2, 0, 0]),
            ('┃', [0, 0, 2, 2]),
            ('┏', [0, 2, 0, 2]),
            ('┓', [2, 0, 0, 2]),
            ('┗', [0, 2, 2, 0]),
            ('┛', [2, 0, 2, 0]),
            ('╸', [2, 0, 0, 0]),
            ('╺', [0, 2, 0, 0]),
            ('╹', [0, 0, 2, 0]),
            ('╻', [0, 0, 0, 2]),
        ] {
            let across = |x: usize, w: i32| w > 0 && (x as i32 - GW as i32 / 2).abs() < w;
            let down = |y: usize, w: i32| w > 0 && (y as i32 - GH as i32 / 2).abs() < w;
            let [l, r, u, d] = arms;
            let s = shape(|x, y| {
                let (left, right) = (x <= GW / 2, x >= GW / 2);
                let (up, dn) = (y <= GH / 2, y >= GH / 2);
                (down(y, if left { l } else { 0 }) || down(y, if right { r } else { 0 }))
                    || (across(x, if up { u } else { 0 }) || across(x, if dn { d } else { 0 }))
            });
            g.push((ch, s));
        }
        // corner to corner, a light line's width
        let rise = |x: usize, y: usize| (16.0 * (x as f32 + 0.5) + 8.0 * (y as f32 + 0.5) - 128.0).abs() / 17.9 < 0.75;
        let fall = |x: usize, y: usize| (16.0 * (x as f32 + 0.5) - 8.0 * (y as f32 + 0.5)).abs() / 17.9 < 0.75;
        g.push(('╱', shape(rise)));
        g.push(('╲', shape(fall)));
        g.push(('╳', shape(|x, y| rise(x, y) || fall(x, y))));
        g
    })
}

/// Braille's dots as a terminal draws them, two columns of four, each 2x2
/// points; dot n is the character's bit n.
fn braille_dots() -> &'static [Shape; 8] {
    static D: std::sync::OnceLock<[Shape; 8]> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        const BIT: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
        std::array::from_fn(|n| shape(|x, y| matches!(x % 4, 1 | 2) && matches!(y % 4, 1 | 2) && BIT[x / 4][y / 4] == 1 << n))
    })
}

/// What a Braille character costs over the rest: its dots read as fuzz, so
/// it wins only where nothing solid fits, as in a dithered fade.
const BRAILLE_COST: u32 = 6;

/// How many of a cell's 8x16 points `ch` draws, for tests.
#[cfg(test)]
pub fn points(ch: char) -> u32 {
    match ch as u32 {
        b @ 0x2800..=0x28FF => (0..8).filter(|n| (b - 0x2800) >> n & 1 == 1).map(|n| braille_dots()[n].count_ones()).sum(),
        _ => glyphs().iter().find(|g| g.0 == ch).map_or(0, |g| g.1.count_ones()),
    }
}

/// The cell over the Mac's `x0..x1`, `y0..y1`, if a silhouette is in it.
/// A cell has two colours, and here there can be more (the ink, its shade,
/// what is behind), so every pair is tried as foreground and background
/// with every character, and the one that draws fewest points in the wrong
/// colour wins. A silhouette's point drawn as what is behind counts twice,
/// so thin things (a 'copter's blades) do not vanish.
fn sharp_cell(c: &Canvas, x0: i32, x1: i32, y0: i32, y1: i32) -> Option<(char, Rgb, Rgb)> {
    if c.fore.is_empty() || !(y0..y1).any(|y| (x0..x1).any(|x| c.fore[(y * W + x) as usize].is_some())) {
        return None;
    }
    let at = |i: usize, j: usize| {
        let x = x0 + (2 * i as i32 + 1) * (x1 - x0) / (2 * GW as i32);
        let y = y0 + (2 * j as i32 + 1) * (y1 - y0) / (2 * GH as i32);
        (y * W + x) as usize
    };
    // the points of each colour, whether it is what is behind, and whether
    // it is a silhouette's; what is behind is grouped into near colours,
    // each its average, so a table top and the wall above it stay apart
    let mut cols: Vec<Rgb> = Vec::new();
    let mut back: Vec<bool> = Vec::new();
    let mut bold: Vec<bool> = Vec::new();
    let mut pts: Vec<Shape> = Vec::new();
    let mut sums: Vec<[u32; 3]> = Vec::new();
    for j in 0..GH {
        for i in 0..GW {
            let k = at(i, j);
            let (col, behind, b) = match c.fore[k] {
                Some((f, b)) => (f, false, b),
                None => (c.px[k], true, false),
            };
            let same = |q: usize| back[q] == behind && if behind { dist(cols[q], col) < SAME } else { cols[q] == col && bold[q] == b };
            let n = (0..cols.len()).find(|&q| same(q)).unwrap_or_else(|| {
                cols.push(col);
                back.push(behind);
                bold.push(b);
                pts.push(0);
                sums.push([0; 3]);
                cols.len() - 1
            });
            pts[n] |= 1 << (j * GW + i);
            for (s, v) in sums[n].iter_mut().zip(col) {
                *s += v as u32;
            }
        }
    }
    if back.iter().all(|&b| b) {
        return None;
    }
    for k in 0..cols.len() {
        if back[k] {
            let n = pts[k].count_ones();
            cols[k] = sums[k].map(|s| (s / n) as u8);
        }
    }
    // art has many colours: near ones are one, and at most four are kept,
    // the rest joining the nearest kept of their kind
    let mut order: Vec<usize> = (0..cols.len()).collect();
    order.sort_by_key(|&k| std::cmp::Reverse(pts[k].count_ones()));
    let mut kept: Vec<usize> = Vec::new();
    for k in order {
        match kept.iter().copied().filter(|&q| back[q] == back[k]).min_by_key(|&q| dist(cols[q], cols[k])) {
            Some(q) if dist(cols[q], cols[k]) < SAME || kept.len() >= 4 => {
                let p = pts[k];
                pts[q] |= p;
                pts[k] = 0;
                bold[q] |= bold[k];
            }
            _ => kept.push(k),
        }
    }
    // a point in the wrong colour costs more the further that is from its
    // own, and twice as much if it is a silhouette's point lost to what is
    // behind
    let wrong = |k: usize, drawn: usize| match (k, drawn) {
        _ if k == drawn => 0,
        _ => (1 + dist(cols[k], cols[drawn]) * 8 / (3 * 255 * 255)) * if back[drawn] && bold[k] { 2 } else { 1 },
    };
    let used: Vec<usize> = (0..cols.len()).filter(|&k| pts[k] != 0).collect();
    let mut best = (u32::MAX, ' ', used[0], used[0]);
    for &f in &used {
        for &b in &used {
            if f == b && used.len() > 1 {
                continue;
            }
            let cost = |s: Shape, drawn: usize| used.iter().map(|&k| (pts[k] & s).count_ones() * wrong(k, drawn)).sum::<u32>();
            for &(ch, s) in glyphs() {
                let e = cost(s, f) + cost(!s, b);
                if e < best.0 {
                    best = (e, ch, f, b);
                }
            }
            // the best Braille, dot by dot; the points between dots are b
            let between = !braille_dots().iter().fold(0, |u, d| u | d);
            let (mut e, mut bits) = (BRAILLE_COST + cost(between, b), 0);
            for (n, &d) in braille_dots().iter().enumerate() {
                let (on, off) = (cost(d, f), cost(d, b));
                if on < off {
                    bits |= 1 << n;
                }
                e += on.min(off);
            }
            if bits != 0 && bits != 0xFF && e < best.0 {
                best = (e, char::from_u32(0x2800 + bits).unwrap_or(' '), f, b);
            }
        }
    }
    let (_, ch, f, b) = best;
    Some(if ch == ' ' { (' ', cols[b], cols[b]) } else { (ch, cols[f], cols[b]) })
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

/// Labels as terminal text at their place over the picture.
pub fn draw_labels(buf: &mut Buffer, p: &Placed, labels: &[crate::draw::Label]) {
    let r = p.cells;
    for l in labels {
        let n = l.text.chars().count() as i32;
        let mut cx = l.x * r.width as i32 / W;
        if l.centred {
            cx -= n / 2;
        }
        let cy = (l.y * r.height as i32 / H).clamp(0, r.height as i32 - 1);
        let cx = cx.clamp(0, (r.width as i32 - n).max(0));
        for (i, ch) in l.text.chars().enumerate() {
            let x = cx + i as i32;
            if x >= r.width as i32 {
                break;
            }
            let cell = &mut buf[(r.x + x as u16, r.y + cy as u16)];
            let bg = l.bg.map(rgb).unwrap_or(cell.bg);
            cell.set_char(ch).set_fg(rgb(l.fg)).set_bg(bg).set_style(ratatui::style::Modifier::BOLD);
        }
    }
}
