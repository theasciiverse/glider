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
