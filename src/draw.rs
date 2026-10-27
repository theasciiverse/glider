//! The pictures, as G-PlayUtils.p and G-PlayActive.p draw them in colour:
//! the room's still picture (`ArrangeTiles`, `DrawAllObjects` and its
//! furniture, drawn with QuickDraw rather than art) and every frame's
//! moving parts in `DrawCScene`'s order. The headline bar (room name and
//! number, score, gliders, rubber bands, battery) is drawn last each frame.

use crate::assets::Assets;
use crate::game::*;
use crate::gfx::{self, BLACK, BROWN, Canvas, DK_GRAY, LT_BLUE, LT_BROWN, LT_GREEN, RED, Trace, WHITE, YELLOW};
use crate::house::{Object, Rect};

#[allow(non_snake_case)]
const fn R(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
    Rect::new(left, top, right, bottom)
}

/// srcRect: where each object (and its animation frames) sits in the art.
pub fn src_rect(n: i32) -> Rect {
    match n {
        CEL_VNT => R(0, 84, 48, 96),
        CEL_DCT => R(0, 97, 48, 110),
        FLR_VNT => R(0, 111, 48, 124),
        PAPER => R(0, 125, 48, 146),
        TOASTR => R(0, 147, 38, 174),
        60..=65 => R(304, 84 + (n - 60) * 32, 336, 115 + (n - 60) * 32),
        TEA_KTL => R(0, 175, 41, 205),
        LFT_FAN => R(0, 206, 35, 261),
        RIT_FAN => R(0, 262, 35, 316),
        TABLE => R(48, 23, 112, 45),
        SHREDR => R(48, 46, 112, 70),
        BOOKS => R(48, 71, 112, 126),
        CLOCK => R(112, 0, 144, 29),
        CANDLE => R(112, 30, 144, 51),
        RBR_BND => R(112, 52, 144, 75),
        BALL => R(112, 76, 144, 108),
        FSH_BWL => R(112, 109, 144, 138),
        66..=69 => R(144, 109 + (n - 66) * 17, 160, 125 + (n - 66) * 17),
        GREASE => R(112, 139, 144, 168),
        58 => R(112, 169, 144, 198),
        59 => R(112, 199, 144, 228),
        LIT_SWT => R(142, 0, 160, 26),
        THERMO => R(144, 27, 162, 54),
        OUTLET => R(160, 264, 192, 289),
        51 => R(160, 290, 192, 315),
        52 => R(160, 316, 192, 341),
        PWR_SWT => R(144, 82, 162, 108),
        GUITAR => R(48, 127, 112, 297),
        DRIP => R(192, 42, 208, 55),
        SHELF => R(192, 71, 208, 100),
        BASKET => R(448, 270, 511, 341),
        PAINTG => R(408, 53, 510, 146),
        BATTRY => R(144, 55, 160, 81),
        MAC_TSH => R(256, 209, 301, 267),
        48..=50 => R(144, 189 + (n - 48) * 13, 160, 201 + (n - 48) * 13),
        53..=56 => R(192, (n - 53) * 14, 208, 13 + (n - 53) * 14),
        57 => R(192, 56, 208, 70),
        _ => Rect::default(),
    }
}

/// glideRct: the glider's poses, turning frames, fade masks and flames.
pub fn glide_rct(n: i32) -> Rect {
    match n {
        0..=3 => R(0, n * 21, 48, 20 + n * 21),
        4..=9 => R(208, (n - 4) * 21, 256, 20 + (n - 4) * 21),
        10..=16 => R(414, 53 + (n - 10) * 21, 462, 73 + (n - 10) * 21),
        17..=23 => R(463, 53 + (n - 17) * 21, 511, 73 + (n - 17) * 21),
        24..=27 => R(256, 24 + (n - 24) * 37, 304, 60 + (n - 24) * 37),
        _ => Rect::default(),
    }
}

/// nextPhase: which mask (fading) or frame (turning) a phase shows.
fn next_phase(mode: i32, phase: i32) -> i32 {
    const FADE_IN: [i32; 17] = [10, 11, 10, 11, 12, 11, 12, 13, 12, 13, 14, 13, 14, 15, 14, 15, 16];
    const TURN: [i32; 12] = [4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9];
    let p = phase.max(0) as usize;
    match mode {
        FADING_IN => FADE_IN[p.min(16)],
        FADING_OUT => 26 - FADE_IN[p.min(16)],
        TURN_RT2LF => TURN[p.min(11)],
        TURN_LF2RT => 13 - TURN[p.min(11)],
        _ => 0,
    }
}

/// animateRct: darts, 'copters and balloons, phase -1 crushed or popped.
fn animate_rct(kind: i32, phase: i32) -> Rect {
    match (kind, phase) {
        (0, -1) => R(304, 0, 368, 22),
        (0, _) => R(48, 0, 112, 22),
        (1, -1) => R(304, 276, 336, 308),
        (1, p) => R(160, p * 33, 192, 32 + p * 33),
        (2, -1) => R(304, 309, 336, 341),
        (2, p) => {
            let row = [0, 0, 1, 1, 2, 2, 1, 1][p.clamp(0, 7) as usize];
            R(112, 229 + row * 33, 144, 261 + row * 33)
        }
        _ => Rect::default(),
    }
}

fn band_rct(phase: i32) -> Rect {
    R(192, 155 + phase.clamp(0, 2) * 8, 208, 162 + phase.clamp(0, 2) * 8)
}

const SHADO_RCT: [Rect; 2] = [R(256, 0, 304, 11), R(256, 12, 304, 23)];

// ---------------------------------------------------------------------
// the room's still picture

fn fill_n_frame(c: &mut Canvas, col: [u8; 3], r: Rect) {
    c.fill(r, col);
    c.frame(r, BLACK);
}

/// MoveTo(a); LineTo(b); LineTo(c) with a 1x1 pen.
fn path(c: &mut Canvas, pts: &[(i32, i32)], col: [u8; 3]) {
    for w in pts.windows(2) {
        c.line(w[0].0, w[0].1, w[1].0, w[1].1, 1, 1, col);
    }
}

fn hi_lite(c: &mut Canvas, col: [u8; 3], r: Rect) {
    path(c, &[(r.left + 1, r.top + 1), (r.right - 2, r.top + 1), (r.right - 2, r.bottom - 2)], col);
}

fn lo_lite(c: &mut Canvas, r: Rect) {
    path(c, &[(r.left - 1, r.top), (r.left - 1, r.bottom), (r.right - 1, r.bottom)], LT_BROWN);
    path(c, &[(r.left, r.top - 1), (r.right, r.top - 1), (r.right, r.bottom)], DK_GRAY);
}

fn draw_table(c: &mut Canvas, a: &Assets, r: Rect) {
    c.fill(r, BROWN);
    c.frame(r, BLACK);
    c.line(r.left + 1, r.bottom - 2, r.right - 1, r.bottom - 2, 1, 1, BLACK);
    c.line(r.left + 1, r.top + 1, r.right - 2, r.top + 1, 1, 1, LT_BROWN);
    let shadow = R(r.left, K_FLOOR_VERT, r.right, K_FLOOR_VERT + 20).offset_by((r.top - K_FLOOR_VERT) / 5, 0);
    c.gray_oval(shadow);
    let mid = (r.left + r.right) / 2;
    c.line(mid - 2, r.bottom, mid - 2, K_FLOOR_VERT - 7, 5, 1, BLACK);
    let lit = r.bottom + (r.right - r.left) / 8;
    c.line(mid + 1, lit, mid + 1, K_FLOOR_VERT - 7, 1, 1, WHITE);
    c.line(mid, lit, mid, K_FLOOR_VERT - 7, 1, 1, LT_BROWN);
    let s = src_rect(TABLE);
    c.sprite(a, s, s.at(mid - 31, K_FLOOR_VERT - 7));
}

fn draw_shelf(c: &mut Canvas, a: &Assets, r: Rect) {
    c.fill(r, LT_BROWN);
    c.frame(r, LT_BROWN);
    c.line(r.left + 1, r.bottom - 2, r.right - 1, r.bottom - 2, 1, 1, LT_BROWN);
    c.line(r.left + 1, r.top + 1, r.right - 2, r.top + 1, 1, 1, WHITE);
    let t = Trace::from(r.right, r.bottom - 1).by(-15, 15).to(r.left - 15, r.bottom + 14).by(0, -5).by(15, -15).by(0, 5).to(r.right, r.bottom - 1);
    c.gray_polygon(&t.pts);
    let s = src_rect(SHELF);
    c.sprite(a, s, s.at(r.left + 15, r.bottom - 2));
    c.sprite(a, s, s.at(r.right - 25, r.bottom - 2));
}

fn draw_mirror(c: &mut Canvas, r: Rect) {
    c.fill(r, BROWN);
    c.frame(r, BLACK);
    let i = r.inset_by(3, 3);
    c.fill(i, WHITE);
    c.frame(i, BLACK);
}

fn draw_cabinet(c: &mut Canvas, r: Rect) {
    let off_it;
    if r.bottom > 280 {
        let mut t = r;
        t.bottom -= 5;
        c.fill(t, BROWN);
        c.frame(t, BLACK);
        let mut t = r.inset_by(2, 0);
        t.top = t.bottom - 5;
        c.fill(t, BLACK);
        let t = R(r.left - 2, r.top, r.right + 2, r.top + 7);
        c.fill(t, LT_BROWN);
        c.frame(t, BLACK);
        c.line(t.left + 2, t.bottom, t.right - 3, t.bottom, 1, 1, BLACK);
        let s = Trace::from(r.left, r.top + 5).by(-15, 15).to(r.left - 15, r.bottom - 10).to(r.left, r.bottom).to(r.left, r.top + 5);
        c.gray_polygon(&s.pts);
        off_it = 5;
    } else {
        c.fill(r, BROWN);
        c.frame(r, BLACK);
        let s = Trace::from(r.left, r.top).by(-15, 15).to(r.left - 15, r.bottom + 15).to(r.right - 15, r.bottom + 15).by(15, -15).to(r.left, r.bottom).to(r.left, r.top);
        c.gray_polygon(&s.pts);
        off_it = 0;
    }
    let panel = |c: &mut Canvas, t: Rect| {
        path(c, &[(t.left + 3, t.top + 3), (t.left + 3, t.bottom - 4), (t.right - 4, t.bottom - 4)], LT_BROWN);
        path(c, &[(t.right - 4, t.bottom - 4), (t.right - 4, t.top + 3), (t.left + 3, t.top + 3)], BLACK);
    };
    let panels = r.width() / 48;
    if panels == 0 {
        let t = r.inset_by(5, 5 + off_it);
        c.frame(t, BLACK);
        panel(c, t);
    } else {
        let width = (r.width() - (panels + 1) * 5) / panels;
        let mut t = R(r.left + 5, r.top + 5 + off_it, r.left + 5 + width, r.bottom - 5 - off_it);
        for _ in 0..panels {
            panel(c, t);
            t.offset(width + 5, 0);
        }
    }
}

fn draw_window(c: &mut Canvas, o: &Object) {
    let r = o.bound;
    let s = Trace::from(r.left, r.top)
        .by(-10, 10)
        .by(0, 5)
        .by(5, 5)
        .to(r.left - 5, r.bottom - 10)
        .by(-5, 5)
        .by(0, 5)
        .by(5, 5)
        .to(r.right - 5, r.bottom + 5)
        .to(r.right, r.bottom)
        .to(r.left, r.bottom)
        .to(r.left, r.top);
    c.gray_polygon(&s.pts);
    fill_n_frame(c, BROWN, r);
    hi_lite(c, LT_BROWN, r);
    let t = R(r.left - 4, r.top, r.right + 4, r.top + 6);
    fill_n_frame(c, BROWN, t);
    hi_lite(c, LT_BROWN, t);
    fill_n_frame(c, BROWN, R(r.left - 2, r.top + 6, r.right + 2, r.top + 10));
    let t = R(r.left - 4, r.bottom - 6, r.right + 4, r.bottom);
    fill_n_frame(c, BROWN, t);
    hi_lite(c, LT_BROWN, t);
    let t = R(r.left - 2, r.bottom - 10, r.right + 2, r.bottom - 5);
    fill_n_frame(c, BROWN, t);
    hi_lite(c, LT_BROWN, t);
    let inner = r.inset_by(8, 16);
    fill_n_frame(c, BROWN, inner);
    lo_lite(c, inner);
    let mid = (r.bottom + r.top) / 2;
    let pane = |c: &mut Canvas, mut t: Rect| {
        fill_n_frame(c, BROWN, t);
        t.inset(6, 6);
        lo_lite(c, t);
        t.inset(2, 2);
        lo_lite(c, t);
        t.inset(2, 2);
        fill_n_frame(c, BLACK, t);
        lo_lite(c, t);
    };
    let mut t = inner;
    t.bottom = mid + 2;
    pane(c, t);
    let mut t = inner;
    t.top = mid + 2;
    c.fill(t, BLACK);
    let mut t = inner;
    t.top = mid - 2;
    if o.is_on {
        t.offset(0, 26 - (r.bottom - r.top) / 2);
    }
    pane(c, t);
}

/// How the pictures are drawn: Calhoun's art, or the silhouette view
/// (softened backgrounds, flat furniture, objects in solid colours by
/// role), which reads better at terminal sizes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Look {
    Original,
    Silhouette,
}

/// Silhouette colours. Hazards warm, prizes bright and cool, the house's
/// own things in muted tones, the glider brightest.
pub mod ink {
    pub const GLIDER: [u8; 3] = [250, 250, 255];
    pub const SHADOW: [u8; 3] = [16, 16, 24];
    pub const WOOD: [u8; 3] = [150, 100, 55];
    pub const SHELF: [u8; 3] = [185, 145, 90];
    pub const FRAME: [u8; 3] = [125, 82, 45];
    pub const GLASS: [u8; 3] = [150, 175, 190];
    pub const FLAME: [u8; 3] = [255, 150, 40];
    pub const SPARK: [u8; 3] = [130, 225, 255];
    pub const DANGER: [u8; 3] = [230, 70, 60];
    pub const TOAST: [u8; 3] = [220, 165, 95];
    pub const FISH: [u8; 3] = [255, 140, 40];
    pub const WATER: [u8; 3] = [90, 160, 255];
    pub const DART: [u8; 3] = [235, 235, 235];
    pub const BALLOON: [u8; 3] = [245, 80, 120];
    pub const BAND: [u8; 3] = [235, 175, 95];
}

/// Objects drawn from their art even in the silhouette view: the house's
/// own things, whose look tells what they are (books, a clock, a battery).
/// Only what moves or hurts is a silhouette.
fn keeps_art(kind: i32) -> bool {
    matches!(
        kind,
        CLOCK | PAPER | BATTRY | RBR_BND | FLR_VNT | CEL_VNT | CEL_DCT | LFT_FAN | RIT_FAN | CANDLE | OUTLET | TOASTR | TEA_KTL | FSH_BWL | GREASE | SHREDR
            | BOOKS | GUITAR | LIT_SWT | THERMO | PWR_SWT | BASKET | MAC_TSH | DRIP | 58 | 59
    )
}

/// An object's silhouette colour.
pub fn ink_for(kind: i32) -> [u8; 3] {
    match kind {
        CLOCK => [255, 205, 60],
        PAPER => [210, 235, 255],
        BATTRY => [120, 235, 95],
        RBR_BND => ink::BAND,
        FLR_VNT | CEL_VNT | CEL_DCT => [100, 118, 135],
        LFT_FAN | RIT_FAN => [135, 155, 175],
        CANDLE => [235, 225, 190],
        48..=50 => ink::FLAME,
        OUTLET => [115, 115, 125],
        51 | 52 => ink::SPARK,
        TOASTR | TEA_KTL => [195, 200, 210],
        60..=65 => ink::TOAST,
        FSH_BWL => [130, 190, 230],
        66..=69 => ink::FISH,
        BALL => ink::DANGER,
        DRIP | 53..=57 => ink::WATER,
        GREASE | 58 | 59 => [150, 140, 60],
        SHREDR => [150, 150, 160],
        BOOKS => [170, 75, 60],
        GUITAR => [195, 85, 50],
        LIT_SWT | THERMO | PWR_SWT => [225, 215, 190],
        BASKET => [110, 170, 90],
        MAC_TSH => [220, 210, 180],
        TABLE => ink::WOOD,
        SHELF => ink::SHELF,
        _ => [180, 180, 180],
    }
}

/// A sprite through a mask, as art or as a silhouette in `col`.
fn paint(c: &mut Canvas, a: &Assets, look: Look, col: [u8; 3], src: Rect, msk: Rect, dst: Rect) {
    c.ink = (look == Look::Silhouette).then_some(col);
    c.copy_mask(a, src, msk, dst);
    c.ink = None;
}

fn sprite(c: &mut Canvas, a: &Assets, look: Look, col: [u8; 3], src: Rect, dst: Rect) {
    paint(c, a, look, col, src, src, dst);
}

/// A room object: its art, or (if it is not one of the house's own
/// things) its silhouette.
fn object(c: &mut Canvas, a: &Assets, look: Look, kind: i32, src: Rect, dst: Rect) {
    let look = if keeps_art(kind) { Look::Original } else { look };
    sprite(c, a, look, ink_for(kind), src, dst);
}

fn flat_table(c: &mut Canvas, a: &Assets, r: Rect) {
    c.fill(r, ink::WOOD);
    let mid = (r.left + r.right) / 2;
    c.line(mid - 2, r.bottom, mid - 2, K_FLOOR_VERT - 7, 5, 1, ink::WOOD);
    let s = src_rect(TABLE);
    sprite(c, a, Look::Silhouette, ink::WOOD, s, s.at(mid - 31, K_FLOOR_VERT - 7));
}

fn flat_shelf(c: &mut Canvas, a: &Assets, r: Rect) {
    c.fill(r, ink::SHELF);
    let s = src_rect(SHELF);
    sprite(c, a, Look::Silhouette, ink::SHELF, s, s.at(r.left + 15, r.bottom - 2));
    sprite(c, a, Look::Silhouette, ink::SHELF, s, s.at(r.right - 25, r.bottom - 2));
}

fn flat_window(c: &mut Canvas, o: &Object) {
    let r = o.bound;
    c.fill(R(r.left - 4, r.top, r.right + 4, r.bottom), ink::FRAME);
    let inner = r.inset_by(8, 16);
    c.fill(inner, BLACK);
    let mid = (r.bottom + r.top) / 2;
    // the sash between the panes, moved up when the window is open
    let mut sash = R(inner.left, mid - 2, inner.right, mid + 4);
    if o.is_on {
        sash.offset(0, 26 - (r.bottom - r.top) / 2);
    }
    c.fill(sash, ink::FRAME);
}

/// The room as it stands, without anything that moves: LoadABackground,
/// ArrangeTiles and DrawAllObjects.
pub fn room_picture(a: &Assets, g: &Game, look: Look) -> Canvas {
    let mut c = Canvas::new();
    let room = g.room();
    let objects = &room.objects[..room.n_objects as usize];
    if g.lights_out {
        c.fill(gfx::WHOLE, BLACK);
        for o in objects.iter().filter(|o| o.kind == LIT_SWT) {
            object(&mut c, a, look, LIT_SWT, src_rect(LIT_SWT), o.bound);
        }
        return c;
    }
    // LoadABackground's substitutes for pictures a graphics file lacks
    let id = room.back_pict;
    let bg = a.backgrounds.get(&id).or_else(|| a.backgrounds.get(&200)).expect("backgrounds 200 to 209");
    for (i, &panel) in room.tile_order.iter().enumerate() {
        let i = i as i32;
        c.copy_bits(bg, R(panel * 64, 0, panel * 64 + 64, 342), R(i * 64, 0, i * 64 + 64, 342));
    }
    let flat = look == Look::Silhouette;
    if flat {
        // stairs and paintings are part of the house, not things to read
        for o in objects {
            match o.kind {
                PAINTG => c.copy_bits(&a.objects, src_rect(PAINTG), o.bound),
                UP_STAR => c.copy_bits(&a.stair_up, R(0, 0, a.stair_up.w, a.stair_up.h), o.bound),
                DN_STAR => c.copy_bits(&a.stair_down, R(0, 0, a.stair_down.w, a.stair_down.h), o.bound),
                _ => {}
            }
        }
        c.soften(4, 0.5);
    }
    for o in objects {
        let s = src_rect(o.kind);
        match o.kind {
            TABLE if flat => flat_table(&mut c, a, o.bound),
            SHELF if flat => flat_shelf(&mut c, a, o.bound),
            CABNET if flat => c.fill(o.bound, ink::WOOD),
            WINDOW if flat => flat_window(&mut c, o),
            MIRROR if flat => {
                c.fill(o.bound, ink::FRAME);
                c.fill(o.bound.inset_by(3, 3), ink::GLASS);
            }
            PAINTG | UP_STAR | DN_STAR if flat => {}
            TABLE => draw_table(&mut c, a, o.bound),
            SHELF => draw_shelf(&mut c, a, o.bound),
            CABNET => draw_cabinet(&mut c, o.bound),
            BOOKS | FLR_VNT..=CANDLE | LFT_FAN | RIT_FAN | LIT_SWT..=GUITAR | DRIP | TOASTR | FSH_BWL | TEA_KTL | BASKET | MAC_TSH => {
                object(&mut c, a, look, o.kind, s, o.bound)
            }
            GREASE => {
                if !o.is_on {
                    let b = o.bound;
                    c.line(b.right, b.bottom - 2, o.amount, b.bottom - 2, 2, 2, BLACK);
                    object(&mut c, a, look, 59, src_rect(59), b);
                }
            }
            CLOCK | PAPER | BATTRY | RBR_BND => {
                if o.amount > 0 {
                    object(&mut c, a, look, o.kind, s, o.bound);
                }
            }
            WINDOW => draw_window(&mut c, o),
            MIRROR => draw_mirror(&mut c, o.bound),
            PAINTG => c.copy_bits(&a.objects, s, o.bound),
            UP_STAR => c.copy_bits(&a.stair_up, R(0, 0, a.stair_up.w, a.stair_up.h), o.bound),
            DN_STAR => c.copy_bits(&a.stair_down, R(0, 0, a.stair_down.w, a.stair_down.h), o.bound),
            _ => {}
        }
    }
    c
}

/// A straight edge of the house for the box-drawing overlay: furniture
/// drawn from the room's rectangles, so its geometry is exact.
#[derive(Clone, Copy, Debug)]
pub enum Edge {
    /// A horizontal bar (table top, shelf) along the middle of the rect.
    Bar(Rect),
    /// A vertical post (table leg) along the middle of the rect.
    Post(Rect),
    /// An outline (cabinet, window, mirror).
    Outline(Rect),
}

/// The room's straight edges, for the silhouette view's line overlay.
pub fn edges(g: &Game) -> Vec<(Edge, [u8; 3])> {
    let mut out = Vec::new();
    if g.lights_out {
        return out;
    }
    let room = g.room();
    let light = |c: [u8; 3]| [c[0].saturating_add(60), c[1].saturating_add(55), c[2].saturating_add(45)];
    for o in &room.objects[..room.n_objects as usize] {
        let r = o.bound;
        match o.kind {
            TABLE => {
                let mid = (r.left + r.right) / 2;
                out.push((Edge::Post(R(mid - 2, r.bottom, mid + 3, K_FLOOR_VERT - 7)), light(ink::WOOD)));
                out.push((Edge::Bar(r), light(ink::WOOD)));
            }
            SHELF => out.push((Edge::Bar(r), light(ink::SHELF))),
            CABNET => out.push((Edge::Outline(r), light(ink::WOOD))),
            WINDOW => out.push((Edge::Outline(R(r.left - 4, r.top, r.right + 4, r.bottom)), light(ink::FRAME))),
            MIRROR => out.push((Edge::Outline(r), light(ink::FRAME))),
            _ => {}
        }
    }
    out
}

// ---------------------------------------------------------------------
// every frame

/// The glider in its current pose at `dest`.
fn draw_glider(c: &mut Canvas, a: &Assets, look: Look, g: &Game, dest: Rect) {
    let gl = &g.glider;
    let ink = ink::GLIDER;
    if g.life_normal {
        sprite(c, a, look, ink, glide_rct(gl.src_num), dest);
        return;
    }
    match gl.mode {
        FADING_IN | FADING_OUT => {
            let mut m = next_phase(gl.mode, gl.phase);
            if !gl.is_right {
                m += 7;
            }
            paint(c, a, look, ink, glide_rct(gl.src_num), glide_rct(m), dest);
        }
        TURN_RT2LF | TURN_LF2RT => {
            let f = next_phase(gl.mode, gl.phase);
            sprite(c, a, look, ink, glide_rct(f), dest);
        }
        // on fire, the flames show in the silhouette view
        BURNING => sprite(c, a, look, ink::FLAME, glide_rct(gl.src_num), dest),
        ASCENDING | DESCENDING => sprite(c, a, look, ink, glide_rct(gl.src_num), dest),
        SHREDDING => {
            let h = dest.height();
            sprite(c, a, look, ink, R(256, 208 - h, 304, 208), dest);
        }
        _ => sprite(c, a, look, ink, glide_rct(gl.src_num), dest),
    }
}

/// One frame: the room picture with everything that moves on it.
pub fn scene(a: &Assets, g: &Game, room: &Canvas, look: Look) -> Canvas {
    let mut c = room.clone();
    let l = &g.lightning;
    if !g.window_rgn.is_empty() && (l.what_phase == 1 || l.what_phase == 3) {
        // XORed twice and once more: the bolt shows on phases 1 and 3
        let b = &l.bolts[l.which_bolt.clamp(1, 3)];
        for &r in &g.window_rgn {
            c.clip_to(r);
            let (mut x, mut y) = (b[0][0], b[0][1]);
            for j in 1..8 {
                c.line(x, y, b[j][0], b[j][1], 2, 2, WHITE);
                (x, y) = (b[j][0], b[j][1] + 2);
            }
        }
        c.unclip();
    }
    if let Some(m) = g.mirror_rgn {
        c.clip_to(m);
        let gl = &g.glider;
        sprite(&mut c, a, look, [200, 205, 215], glide_rct(gl.src_num), gl.dest.offset_by(-32, -32));
        c.unclip();
    }
    for cy in &g.cycles {
        let er = g.event_rect[cy.tied_to];
        match cy.kind {
            CANDLE => sprite(&mut c, a, look, ink::FLAME, src_rect(cy.position as i32), cy.whole),
            OUTLET | GREASE => {
                object(&mut c, a, look, cy.accel, src_rect(cy.accel), cy.whole);
                if cy.kind == GREASE && cy.velocity > cy.whole.right + 1 {
                    let y = cy.whole.bottom - 2;
                    c.line(cy.whole.right, y, cy.velocity, y, 2, 2, BLACK);
                }
            }
            BALL => sprite(&mut c, a, look, ink::DANGER, src_rect(BALL), er),
            DRIP => object(&mut c, a, look, DRIP, src_rect(cy.phase), er),
            TOASTR => {
                if cy.phase != 0 {
                    if let Some(t) = g.toast_rgn {
                        c.clip_to(t);
                    }
                    sprite(&mut c, a, look, ink::TOAST, src_rect(cy.phase), er);
                    c.unclip();
                }
            }
            FSH_BWL => sprite(&mut c, a, look, ink::FISH, src_rect(if cy.phase != 0 { cy.phase } else { 69 }), er),
            _ => {}
        }
    }
    let gl = &g.glider;
    sprite(&mut c, a, look, ink::SHADOW, SHADO_RCT[if gl.is_right { 0 } else { 1 }], gl.shado_dest);
    draw_glider(&mut c, a, look, g, gl.dest);
    for an in &g.animates {
        if !an.unseen {
            let col = [ink::DART, [205, 205, 225], ink::BALLOON][an.kind.clamp(0, 2) as usize];
            sprite(&mut c, a, look, col, animate_rct(an.kind, an.phase), an.dest);
        }
    }
    if g.band_borne {
        sprite(&mut c, a, look, ink::BAND, band_rct(g.band.phase), g.band.dest);
    }
    // the silhouette view shows the headline's figures as terminal text
    if look == Look::Original {
        headline(&mut c, a, g);
    }
    c
}

/// Text over the picture, drawn as terminal text rather than in the
/// picture: at terminal sizes the Mac's 12-point letters are a smudge.
/// (x, y) is in the room, y the middle of the line.
#[derive(Clone, Debug)]
pub struct Label {
    pub x: i32,
    pub y: i32,
    pub text: String,
    pub fg: [u8; 3],
    /// Its own background, or the picture's.
    pub bg: Option<[u8; 3]>,
    pub centred: bool,
}

/// The game's labels: the points floating up from a prize, yellow at
/// first, then blinking blue and red as FloatTheScore and DrawCScene do.
pub fn labels(g: &Game) -> Vec<Label> {
    let mut out = Vec::new();
    if let Some(f) = &g.float_points {
        let fg = if f.fresh {
            YELLOW
        } else if g.binary_flip == 0 {
            [100, 150, 255]
        } else {
            RED
        };
        out.push(Label { x: f.where_r.left, y: f.where_r.bottom - 6, text: f.says.clone(), fg, bg: None, centred: false });
    }
    out
}

/// A box of the headline bar: erased, framed, shadowed, then inverted
/// inside, which leaves it black with a white rim.
fn headline_box(c: &mut Canvas, r: Rect) {
    c.fill(r, WHITE);
    c.frame(r, BLACK);
    path(c, &[(r.left - 1, r.top + 2), (r.left - 1, r.bottom), (r.right - 2, r.bottom)], BLACK);
    c.invert(r.inset_by(2, 2));
}

/// DrawHeadline, PopTheScore, the glider tokens and the band and battery
/// counters, from the game's state.
pub fn headline(c: &mut Canvas, a: &Assets, g: &Game) {
    let name = crate::mac::mac_roman(&g.room().name);
    headline_box(c, R(3, 2, 162, 23));
    c.text(&a.font, &name, 7, 17, RED);

    // the room number on little cards
    let src = R(36, 318, 54, 341);
    let mut card = R(165, 1, 183, 24);
    let rp = g.rooms_passed.max(0);
    c.copy_bits(&a.objects, src, card);
    let shown = if rp > 99 {
        rp / 100
    } else if rp > 9 {
        rp / 10
    } else {
        rp
    };
    c.text(&a.font, &shown.to_string(), card.left + 5, card.bottom - 6, BLACK);
    if rp > 9 {
        card.offset(20, 0);
        c.copy_bits(&a.objects, src.offset_by(18, 0), card);
        let shown = if rp > 99 { rp / 10 - rp % 10 } else { rp % 10 };
        c.text(&a.font, &shown.to_string(), card.left + 6, card.bottom - 6, BLACK);
        if rp > 99 {
            card.offset(20, 0);
            c.copy_bits(&a.objects, src, card);
            c.text(&a.font, &(rp % 100).to_string(), card.left + 5, card.bottom - 6, BLACK);
        }
    }

    headline_box(c, R(302, 2, 510, 23));
    if g.mortals > 1 {
        let shown = if g.mortals > 3 {
            c.text(&a.font, &(g.mortals - 1).to_string(), 384, 18, RED);
            3
        } else {
            g.mortals - 1
        };
        let mut t = R(470, 5, 505, 20);
        for _ in 0..shown {
            c.sprite(a, R(0, 318, 35, 333), t);
            t.offset(-37, 0);
        }
    }
    headline_box(c, R(220, 2, 298, 23));
    c.fill(R(222, 4, 296, 21), BLACK);
    c.text(&a.font, &g.roll_score.to_string(), 224, 17, LT_BLUE);
    let gl = &g.glider;
    if gl.bands > 0 {
        c.sprite(a, R(36, 301, 45, 316), R(359, 5, 368, 20));
        let s = if gl.bands < 100 { gl.bands.to_string() } else { "++".into() };
        c.text(&a.font, &s, 346, 18, LT_GREEN);
    }
    if gl.energy > 0 {
        c.sprite(a, R(46, 301, 55, 316), R(326, 5, 335, 20));
        let s = if gl.energy < 1000 { gl.energy.to_string() } else { "+++".into() };
        c.text(&a.font, &s, 308, 18, YELLOW);
    }
}
