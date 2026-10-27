//! The transitions the Pascal drew straight to the screen, played tick by
//! tick (60 a second): the room scrolls of G-PlaySetUp.p, DissBlocks of
//! G-GlobalUtils.p, the time bonus, ClosingAnimation (the farm, the
//! twister, the tiny glider) and GameOver (the score and the falling
//! letters).

use crate::assets::Assets;
use crate::game::{Fx, snd};
use crate::gfx::{BLACK, Canvas, WHITE, YELLOW};
use crate::house::Rect;

#[allow(non_snake_case)]
const fn R(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
    Rect::new(left, top, right, bottom)
}

/// DissBlocks' order: a 10-bit shift register with taps $240, each value a
/// 16-pixel block, (value mod 32, value div 32). 1023 values; the blocks
/// below the picture are skipped, and block 0 comes with the final copy.
fn diss_order() -> Vec<Rect> {
    let mut out = Vec::new();
    let mut value: u32 = 1;
    loop {
        if value & 1 != 0 {
            value = (value >> 1) ^ 0x240;
        } else {
            value >>= 1;
        }
        let (h, v) = ((value % 32) as i32 * 16, (value / 32) as i32 * 16);
        if v < 342 {
            out.push(R(h, v, h + 16, v + 16));
        }
        if value == 1 {
            break;
        }
    }
    out
}

/// The GAME OVER letters in the art: G a m e O v r.
const LETTERS: [Rect; 7] = [
    R(375, 33, 409, 78),
    R(375, 77, 409, 122),
    R(375, 121, 409, 166),
    R(375, 165, 409, 210),
    R(375, 209, 409, 254),
    R(375, 253, 409, 298),
    R(375, 297, 409, 342),
];
/// G a m e O v e r
const SPELL: [usize; 8] = [0, 1, 2, 3, 4, 5, 3, 6];

pub struct Show {
    pub kind: Fx,
    t: i32,
    from: Canvas,
    to: Canvas,
    pub canvas: Canvas,
    seed: u32,
    blocks: Vec<Rect>,
    // ClosingAnimation
    twister: Rect,
    tiny: Rect,
    bolt: Option<Vec<(i32, i32)>>,
    // GameOver's falling row
    row: Rect,
}

impl Show {
    /// `from` is the picture on the screen now, `to` what the transition
    /// brings in (the new room's still picture for scrolls and dissolves).
    pub fn new(kind: Fx, a: &Assets, from: Canvas, to: Canvas, score: i64, seed: u32) -> Show {
        let mut canvas = from.clone();
        let mut to = to;
        match &kind {
            Fx::TimeBonus(b) => {
                let r = R(190, 100, 312, 117);
                canvas.fill(r, BLACK);
                canvas.frame(r.inset_by(1, 1), WHITE);
                canvas.text(&a.font, &format!("Time Bonus = {b}"), r.left + 3, r.bottom - 4, YELLOW);
            }
            Fx::Closing => {
                to = Canvas::new();
                to.copy_bits(&a.farm, R(0, 0, 512, 342), R(0, 0, 512, 342));
            }
            Fx::GameOver => {
                to = Canvas::new();
                to.fill(R(0, 0, 512, 342), BLACK);
                to.text(&a.big_font, &score.to_string(), 120, 80, YELLOW);
            }
            _ => {}
        }
        Show {
            kind,
            t: 0,
            from,
            to,
            canvas,
            seed: seed.max(1),
            blocks: diss_order(),
            twister: R(230, 138, 278, 200),
            tiny: R(420, 90, 441, 100),
            bolt: None,
            row: Rect::default(),
        }
    }

    fn randomize(&mut self, range: i32) -> i32 {
        self.seed = ((self.seed as u64 * 16807) % 0x7FFF_FFFF) as u32;
        let r = (self.seed as u16 as i16 as i32).abs();
        (r as i64 * range as i64 / 32768) as i32
    }

    /// How long it lasts, in ticks.
    pub fn len(&self) -> i32 {
        match self.kind {
            Fx::ScrollForward | Fx::ScrollBackward => crate::game::SCROLL_SIDE_TICKS as i32,
            Fx::ScrollUp | Fx::ScrollDown => crate::game::SCROLL_UPDOWN_TICKS as i32,
            Fx::Dissolve => crate::game::DISSOLVE_TICKS as i32,
            Fx::TimeBonus(_) => crate::game::TIME_BONUS_TICKS as i32,
            // the dissolve, then 200 passes of 4 ticks
            Fx::Closing => crate::game::DISSOLVE_TICKS as i32 + 800,
            // the dissolve, eight letters 6 ticks apart, 20 rows of 8, 200 ticks
            Fx::GameOver => crate::game::DISSOLVE_TICKS as i32 + 48 + 160 + 200,
        }
    }

    /// Can a key cut it short? (ClosingAnimation and GameOver listen for
    /// the button and Return.)
    pub fn skippable(&self) -> bool {
        matches!(self.kind, Fx::Closing | Fx::GameOver)
    }

    fn dissolve(&mut self, step: i32, of: i32) {
        let n = self.blocks.len();
        let upto = (n as i32 * step / of.max(1)).min(n as i32) as usize;
        let from = (n as i32 * (step - 1).max(0) / of.max(1)) as usize;
        for i in from..upto {
            let r = self.blocks[i];
            self.canvas.copy_from(&self.to, r);
        }
        if step >= of {
            self.canvas = self.to.clone();
        }
    }

    /// One tick. True when it is over.
    pub fn tick(&mut self, a: &Assets, sounds: &mut Vec<i32>) -> bool {
        self.t += 1;
        let t = self.t;
        let d = crate::game::DISSOLVE_TICKS as i32;
        match self.kind.clone() {
            Fx::ScrollForward | Fx::ScrollBackward => {
                let k = t.min(32);
                let mut c = self.from.clone();
                let w = 16 * k;
                if self.kind == Fx::ScrollForward {
                    c.copy_from(&self.to, R(512 - w, 0, 512, 342));
                    c.line(512 - w - 1, 0, 512 - w - 1, 322, 1, 1, BLACK);
                } else {
                    c.copy_from(&self.to, R(0, 0, w, 342));
                    c.line(w, 0, w, 322, 1, 1, BLACK);
                }
                self.canvas = c;
            }
            Fx::ScrollUp | Fx::ScrollDown => {
                let k = t.min(19);
                let mut c = self.from.clone();
                let h = 18 * k;
                if self.kind == Fx::ScrollUp {
                    c.copy_from(&self.to, R(0, 0, 512, h));
                    c.line(0, h, 512, h, 1, 1, BLACK);
                } else {
                    c.copy_from(&self.to, R(0, 342 - h, 512, 342));
                    c.line(0, 342 - h - 1, 512, 342 - h - 1, 1, 1, BLACK);
                }
                self.canvas = c;
            }
            Fx::Dissolve => self.dissolve(t, d),
            Fx::TimeBonus(_) => {}
            Fx::Closing => {
                if t <= d {
                    self.dissolve(t, d);
                } else if (t - d) % 4 == 1 {
                    let i = (t - d) / 4 + 1;
                    let mut which = i % 4;
                    if i % 4 == 0 {
                        which += 1;
                        self.twister.offset(-1, 0);
                    }
                    self.tiny.offset(-2, -(i % 2));
                    let twister_src = [R(208, 126, 256, 188), R(208, 189, 256, 251), R(208, 252, 256, 314), R(256, 268, 304, 330)];
                    let mut c = self.to.clone();
                    c.sprite(a, twister_src[which as usize], self.twister);
                    c.sprite(a, R(235, 315, 256, 325), self.tiny);
                    // the thunder comes as the last bolt is taken away
                    if self.bolt.take().is_some() {
                        sounds.push(snd::THUNDER);
                    }
                    if self.randomize(20) == 0 {
                        // CastBolt: nine white segments under the cloud
                        let mut start = self.randomize(100) + 200;
                        let mut pts = Vec::new();
                        for k in 0..=8 {
                            let finish = self.randomize(7) - 3 + start;
                            c.line(start, k * 8 + 141, finish, (k + 1) * 8 + 140, 1, 1, WHITE);
                            pts.push((start, finish));
                            start = finish;
                        }
                        self.bolt = Some(pts);
                    }
                    self.canvas = c;
                }
            }
            Fx::GameOver => {
                if t <= d {
                    self.dissolve(t, d);
                } else if t <= d + 48 {
                    // a letter every 6 ticks
                    let k = t - d - 1;
                    if k % 6 == 0 {
                        let n = (k / 6) as usize;
                        sounds.push(snd::SCORE_TICK);
                        let dest = R(113 + 36 * n as i32, 100, 147 + 36 * n as i32, 145);
                        self.canvas.copy_bits(&a.objects, LETTERS[SPELL[n]], dest);
                    }
                } else if t <= d + 48 + 160 {
                    // twenty rows, each lower and shakier, drawn over the last
                    // a letter a tick
                    let k = t - d - 49;
                    let (i, l) = (k / 8 + 1, (k % 8) as usize);
                    if l == 0 {
                        self.row = R(113, 100 + i * 8, 147, 145 + i * 8);
                    }
                    let j = self.randomize(i * 2 + 1) - i;
                    self.row.offset(j, 0);
                    let dest = self.row;
                    self.canvas.copy_bits(&a.objects, LETTERS[SPELL[l]], dest);
                    self.row.offset(36, 0);
                }
            }
        }
        t >= self.len()
    }
}
