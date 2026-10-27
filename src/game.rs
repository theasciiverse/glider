//! The game, translated from Glider 4.05's Pascal: `Coordinate` and what it
//! calls in G-PlayActive.p, the room set-up of G-PlayUtils.p
//! (`ExtractEventRects`, `ResetGlider`, `ResetAnimate`, `ReadyRoom`) and
//! the room changes of G-PlaySetUp.p. Names follow the Pascal so the two
//! can be read side by side.
//!
//! Drawing is not done here: what the Pascal drew straight to the screen
//! is either derived from this state each frame (`draw.rs`) or handed to
//! the front end as an [`Fx`] (scrolling to the next room, the time bonus,
//! game over), which also takes the time it took on a Mac out of the
//! tick count, as it did there.
//!
//! Time is the Mac's `TickCount`, 60 a second; a frame is two ticks.

use crate::house::{House, Library, MAX_ROOMS, Object, Rect, names_a_file};

// room object codes
pub const TABLE: i32 = 1;
pub const SHELF: i32 = 2;
pub const BOOKS: i32 = 3;
pub const CABNET: i32 = 4;
pub const EXT_RCT: i32 = 5;
pub const OBS_RCT: i32 = 6;
pub const FLR_VNT: i32 = 8;
pub const CEL_VNT: i32 = 9;
pub const CEL_DCT: i32 = 10;
pub const CANDLE: i32 = 11;
pub const LFT_FAN: i32 = 12;
pub const RIT_FAN: i32 = 13;
pub const CLOCK: i32 = 16;
pub const PAPER: i32 = 17;
pub const GREASE: i32 = 18;
pub const BNS_RCT: i32 = 19;
pub const BATTRY: i32 = 20;
pub const RBR_BND: i32 = 21;
pub const LIT_SWT: i32 = 24;
pub const OUTLET: i32 = 25;
pub const THERMO: i32 = 26;
pub const SHREDR: i32 = 27;
pub const PWR_SWT: i32 = 28;
pub const GUITAR: i32 = 29;
pub const DRIP: i32 = 32;
pub const TOASTR: i32 = 33;
pub const BALL: i32 = 34;
pub const FSH_BWL: i32 = 35;
pub const TEA_KTL: i32 = 36;
pub const WINDOW: i32 = 37;
pub const PAINTG: i32 = 40;
pub const MIRROR: i32 = 41;
pub const BASKET: i32 = 42;
pub const MAC_TSH: i32 = 43;
pub const UP_STAR: i32 = 44;
pub const DN_STAR: i32 = 45;

// glider modes
pub const NORMAL: i32 = 0;
pub const FADING_IN: i32 = 1;
pub const FADING_OUT: i32 = 2;
pub const TURN_RT2LF: i32 = 3;
pub const TURN_LF2RT: i32 = 4;
pub const BURNING: i32 = 5;
pub const ASCENDING: i32 = 6;
pub const DESCENDING: i32 = 7;
pub const SHREDDING: i32 = 8;

// object effects
const IGNORE_IT: i64 = 0;
const CRASH_IT: i64 = 1;
const LIFT_IT: i64 = 2;
const DROP_IT: i64 = 3;
const MOVE_IT: i64 = 4;
const BURN_IT: i64 = 5;
const TURN_IT_LEFT: i64 = 6;
const TURN_IT_RIGHT: i64 = 7;
const AWARD_IT: i64 = 8;
const EXTRA_IT: i64 = 9;
const SLIDE_IT: i64 = 10;
const TRICK_IT: i64 = 11;
const ENERGIZE_IT: i64 = 12;
const BAND_IT: i64 = 13;
const PLAY_IT: i64 = 14;
const LIGHT_IT: i64 = 15;
const ZAP_IT: i64 = 16;
const AIR_ON_IT: i64 = 17;
const SHRED_IT: i64 = 18;
const TOGGLE_IT: i64 = 19;
const SPILL_IT: i64 = 21;
const ASCEND_IT: i64 = 22;
const DESCEND_IT: i64 = 23;
const STEAM_IT: i64 = 24;

// where the glider came in
const WHO_CARES: i32 = 0;
const TOP_OF_ROOM: i32 = 1;
const BOTTOM_OF_ROOM: i32 = 2;
const LEFT_OF_ROOM: i32 = 3;
const RIGHT_OF_ROOM: i32 = 4;

pub const K_CEILING_VERT: i32 = 24;
pub const K_FLOOR_VERT: i32 = 325;
const K_FLOOR_LIMIT: i32 = K_FLOOR_VERT + 5;
const K_MAX_THRUST: i32 = 5;
const K_BONUS_TIME_TO_BEAT: i64 = 256;
const K_MIN_BASS_LOOP: i64 = 14;
const K_BASS_FRACT: i64 = 8;
/// kFastScoreIncrement: the score odometer's step on a fast Mac.
const SCORE_INCREMENT: i64 = 37;

pub const WHOLE_AREA: Rect = Rect::new(0, 0, 512, 342);

/// Ticks the transitions took on a Mac. The scrolls wait a tick per strip
/// (32 strips across, 19 down); the time bonus shows for 30 ticks.
/// DissBlocks has no wait of its own: it took as long as 704 CopyBits
/// calls, which on the Macs of 1991 was a fraction of a second. The 12
/// here is an estimate, not a measurement.
pub const SCROLL_SIDE_TICKS: i64 = 32;
pub const SCROLL_UPDOWN_TICKS: i64 = 19;
pub const TIME_BONUS_TICKS: i64 = 30;
pub const DISSOLVE_TICKS: i64 = 12;
/// ReadyRoom: after the objects' timers are set (ExtractEventRects), the
/// Mac decoded and drew the room's picture (LoadABackground, ArrangeTiles,
/// DrawAllObjects, DrawHeadline) before anything else happened, and the
/// outlets' and toasters' clocks ran on meanwhile. An estimate, see
/// CLAUDE.md.
pub const READY_ROOM_TICKS: i64 = 0;

/// Sounds by their SMSD number.
pub mod snd {
    pub const CRUNCH: i32 = 1;
    pub const BEAM_IN: i32 = 2;
    pub const BLOWER_ON: i32 = 3;
    pub const BOUNCE: i32 = 4;
    pub const CLOCK: i32 = 5;
    pub const POP_DART: i32 = 6;
    pub const DRIP: i32 = 7;
    pub const YOW: i32 = 8;
    pub const ENERGIZE: i32 = 9;
    pub const EXTRA: i32 = 10;
    pub const FIRE_BAND: i32 = 11;
    pub const GET_BAND: i32 = 12;
    pub const WHISTLE: i32 = 13;
    pub const GREASE_FALL: i32 = 14;
    pub const GUITAR: i32 = 15;
    pub const HEY: i32 = 16;
    pub const THUNDER: i32 = 17;
    pub const CLICK: i32 = 18;
    pub const POP_BALLOON: i32 = 19;
    pub const SHRED: i32 = 20;
    pub const SCORE_TICK: i32 = 21;
    pub const TOAST_LAND: i32 = 22;
    pub const TOAST_LAUNCH: i32 = 23;
    pub const ZAP: i32 = 24;
    pub const TEA_KETTLE: i32 = 25;
    pub const PUSH: i32 = 26;
    pub const MUSIC: i32 = 29;
    pub const BASS: i32 = 30;
}

/// A transition the front end shows, in order, before play goes on.
#[derive(Clone, Debug, PartialEq)]
pub enum Fx {
    /// The new room wipes in from the right (ScrollForward), the left
    /// (ScrollBackward), the top (ScrollUp) or the bottom (ScrollDown).
    ScrollForward,
    ScrollBackward,
    ScrollUp,
    ScrollDown,
    /// DissBlocks: the new picture in 16-pixel blocks.
    Dissolve,
    TimeBonus(i64),
    /// The house is finished: the farm, the twister, the tiny glider.
    Closing,
    /// No gliders left, or the house is over: the score and GAME OVER.
    GameOver,
}

/// The glider's controls for one frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct Controls {
    pub left: bool,
    pub right: bool,
    pub energy: bool,
    pub band: bool,
    /// Absolute mouse control: the pointer's x in the room, and the button.
    pub mouse_x: Option<i32>,
    pub button: bool,
}

/// controlMethod: holdKey, dropKey, absMouse (relMouse is not offered).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    HoldKey,
    DropKey,
    AbsMouse,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Glider {
    pub dest: Rect,
    pub old: Rect,
    pub shado_dest: Rect,
    pub touch: Rect,
    pub time_stamp: i64,
    pub mode: i32,
    pub phase: i32,
    pub src_num: i32,
    pub for_vel: i32,
    pub bands: i32,
    pub energy: i32,
    pub is_right: bool,
    pub is_forward: bool,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Animate {
    pub dest: Rect,
    pub hori_off: i32,
    pub vert_off: i32,
    pub kind: i32,
    pub phase: i32,
    pub tick_stamp: i64,
    pub delay: i64,
    pub unseen: bool,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Cycle {
    pub hold: Rect,
    pub whole: Rect,
    pub tied_to: usize,
    pub kind: i32,
    pub phase: i32,
    pub reset: i64,
    pub position: i64,
    pub accel: i32,
    pub velocity: i32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Band {
    pub dest: Rect,
    pub whole: Rect,
    pub phase: i32,
    pub velocity: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Float {
    pub where_r: Rect,
    pub tick_stamp: i64,
    pub says: String,
    /// The first frame is drawn in yellow, then it blinks blue and red.
    pub fresh: bool,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Lightning {
    /// [bolt 1..3][joint 0..7][x, y]
    pub bolts: [[[i32; 2]; 8]; 4],
    pub which_bolt: usize,
    pub what_phase: i32,
    pub what_time: i64,
}

pub struct Game {
    pub lib: Library,
    pub house: House,
    pub house_name: Vec<u8>,
    pub first_house: Vec<u8>,
    pub method: Method,
    pub ticks: i64,
    rand_seed: u32,
    pub ready_room_ticks: i64,
    /// Ticks a frame takes: Coordinate never goes faster than 2, a slow Mac
    /// took longer. For studying houses built on slower machines.
    pub frame_ticks: i64,

    pub playing: bool,
    pub room_at: i32,
    pub rooms_passed: i32,
    pub room_score: i64,
    pub supp_score: i64,
    pub roll_score: i64,
    pub loops_thru_room: i64,
    pub room_visits: [bool; MAX_ROOMS + 1],
    pub score_is_rolling: bool,
    pub mortals: i32,

    pub glider: Glider,
    pub life_normal: bool,
    pub lift_amount: i32,
    pub shift_amount: i32,
    pub sliding: bool,
    hot_rect: Rect,
    pub entered_left: bool,
    side_you_cant_exit: i32,
    didnt_exit_entrance: bool,
    pub binary_flip: i32,

    pub animates: Vec<Animate>,
    pub cycles: Vec<Cycle>,
    pub band: Band,
    pub band_borne: bool,
    pub float_points: Option<Float>,
    pub lightning: Lightning,

    /// eventRect and eventKind, indexes 1 to 16 as in the Pascal.
    pub event_rect: [Rect; 17],
    pub event_kind: [[i64; 3]; 17],
    pub n_objects: usize,

    pub left_is_open: bool,
    pub right_is_open: bool,
    pub lights_out: bool,
    pub air_out: bool,
    pub window_open: bool,
    /// The window's panes, where lightning shows (windowRgn).
    pub window_rgn: Vec<Rect>,
    pub mirror_rgn: Option<Rect>,
    pub toast_rgn: Option<Rect>,

    bass_loop: i64,
    play_bass_time: i64,

    /// The room's still picture must be built again (a new room, the
    /// lights going on, a prize taken).
    pub room_dirty: bool,
    pub sounds: Vec<i32>,
    pub fx: Vec<Fx>,
}

impl Game {
    pub fn new(lib: Library, house_name: &[u8], house: House, method: Method, seed: u32) -> Game {
        Game {
            lib,
            house,
            house_name: house_name.to_vec(),
            first_house: house_name.to_vec(),
            method,
            ticks: 0,
            rand_seed: seed.max(1),
            ready_room_ticks: READY_ROOM_TICKS,
            frame_ticks: 2,
            playing: false,
            room_at: 1,
            rooms_passed: 1,
            room_score: 0,
            supp_score: 0,
            roll_score: 0,
            loops_thru_room: 0,
            room_visits: [false; MAX_ROOMS + 1],
            score_is_rolling: false,
            mortals: 5,
            glider: Glider::default(),
            life_normal: false,
            lift_amount: 0,
            shift_amount: 0,
            sliding: false,
            hot_rect: Rect::NULL,
            entered_left: true,
            side_you_cant_exit: WHO_CARES,
            didnt_exit_entrance: false,
            binary_flip: 0,
            animates: Vec::new(),
            cycles: Vec::new(),
            band: Band::default(),
            band_borne: false,
            float_points: None,
            lightning: Lightning::default(),
            event_rect: [Rect::NULL; 17],
            event_kind: [[0; 3]; 17],
            n_objects: 0,
            left_is_open: false,
            right_is_open: false,
            lights_out: false,
            air_out: false,
            window_open: false,
            window_rgn: Vec::new(),
            mirror_rgn: None,
            toast_rgn: None,
            bass_loop: 0,
            play_bass_time: 0,
            room_dirty: true,
            sounds: Vec::new(),
            fx: Vec::new(),
        }
    }

    /// The Toolbox's Random: the seed times 16807 modulo 2^31 - 1, its low
    /// word as a signed integer.
    fn random(&mut self) -> i32 {
        self.rand_seed = ((self.rand_seed as u64 * 16807) % 0x7FFF_FFFF) as u32;
        let r = self.rand_seed as u16 as i16;
        if r == i16::MIN { 0 } else { r as i32 }
    }

    /// Randomize: 0 to range - 1.
    pub fn randomize(&mut self, range: i32) -> i32 {
        let raw = self.random().abs() as i64;
        ((raw * range as i64) / 32768) as i32
    }

    fn sound(&mut self, n: i32) {
        self.sounds.push(n);
    }

    pub fn room(&self) -> &crate::house::Room {
        &self.house.rooms[self.room_at.clamp(1, MAX_ROOMS as i32) as usize]
    }

    fn room_mut(&mut self) -> &mut crate::house::Room {
        let r = self.room_at.clamp(1, MAX_ROOMS as i32) as usize;
        &mut self.house.rooms[r]
    }

    fn object(&self, i: usize) -> Object {
        self.room().objects[i - 1]
    }

    fn object_mut(&mut self, i: usize) -> &mut Object {
        &mut self.room_mut().objects[i - 1]
    }

    pub fn score(&self) -> i64 {
        self.room_score + self.supp_score
    }

    // ---------------------------------------------------------------
    // StartNewGame (G-IdleInput.p)

    pub fn start(&mut self, ozma: bool) {
        // a house that is not the first of its sequence starts the first
        let first = self.house.first_file.clone();
        if names_a_file(&first)
            && first != self.house_name
            && let Some(h) = self.lib.load(&first)
        {
            self.house = h;
            self.house_name = first;
        }
        self.first_house = self.house_name.clone();
        self.room_at = 1;
        self.rooms_passed = 1;
        self.room_score = 0;
        self.supp_score = 0;
        self.roll_score = 0;
        self.side_you_cant_exit = WHO_CARES;
        self.loops_thru_room = 0;
        self.bass_loop = 0;
        self.play_bass_time = K_BONUS_TIME_TO_BEAT / K_BASS_FRACT + K_MIN_BASS_LOOP;
        self.room_visits = [false; MAX_ROOMS + 1];
        self.mortals = if ozma { 50 } else { 5 };
        self.entered_left = true;
        self.glider.is_right = true;
        self.glider.bands = 0;
        self.glider.for_vel = 4;
        self.glider.energy = 0;
        self.glider.mode = NORMAL;
        self.reset_glider();
        self.playing = true;
        self.score_is_rolling = false;
        self.float_points = None;
        self.ready_room();
        self.fx.push(Fx::Dissolve);
        self.ticks += DISSOLVE_TICKS;
    }

    // ---------------------------------------------------------------
    // Coordinate: one pass of the play loop

    pub fn step(&mut self, c: Controls) {
        let time_is = self.ticks;
        self.lift_amount = 2;
        self.binary_flip ^= 1;
        if self.band_borne {
            self.handle_band();
        }
        if self.life_normal {
            self.search_rects();
            self.get_input(c);
            self.move_glider();
        } else {
            self.do_special();
        }
        if self.playing {
            if !self.animates.is_empty() {
                self.move_animates();
            }
            if !self.cycles.is_empty() {
                self.move_cycle_obs();
            }
            if !self.window_rgn.is_empty() {
                self.monitor_storm();
            }
            self.age_float();
            if self.score_is_rolling {
                self.bump_the_score();
            }
        }
        self.ticks = self.ticks.max(time_is + self.frame_ticks);
        self.loops_thru_room += 1;
        self.bass_loop += 1;
        if self.bass_loop >= self.play_bass_time {
            self.bass_loop = 0;
            self.sound(snd::BASS);
            self.play_bass_time = ((K_BONUS_TIME_TO_BEAT - self.loops_thru_room) / K_BASS_FRACT + K_MIN_BASS_LOOP).max(K_MIN_BASS_LOOP);
        }
    }

    /// What DrawCScene does to the floating score besides drawing it.
    fn age_float(&mut self) {
        let ticks = self.ticks;
        if let Some(f) = &mut self.float_points {
            if ticks > f.tick_stamp {
                self.float_points = None;
            } else if f.fresh {
                f.fresh = false;
            } else {
                f.where_r.offset(2, -1);
            }
        }
    }

    fn float_the_score(&mut self, points: i64) {
        let mut r = self.glider.dest;
        r.top = r.bottom;
        r.bottom = r.top + 12;
        self.float_points = Some(Float { where_r: r, tick_stamp: self.ticks + 120, says: points.to_string(), fresh: true });
    }

    fn start_score_rolling(&mut self) {
        // called before the points are added: the odometer jumps to the
        // old total, then rolls on to the new one
        if self.roll_score < self.score() {
            self.roll_score = self.score();
        }
        self.score_is_rolling = true;
    }

    fn bump_the_score(&mut self) {
        self.roll_score += SCORE_INCREMENT;
        if self.roll_score < self.score() {
            self.sound(snd::SCORE_TICK);
        } else {
            self.score_is_rolling = false;
            self.roll_score = self.score();
        }
    }

    fn crash(&mut self) {
        self.life_normal = false;
        let g = &mut self.glider;
        g.mode = FADING_OUT;
        g.dest.top = g.dest.bottom - 20;
        g.src_num = if g.is_forward { 0 } else { 2 };
    }

    fn bite_it(&mut self) {
        self.sound(snd::CRUNCH);
        self.crash();
    }

    /// A prize taken: it is gone from the room's picture for good.
    fn take_prize(&mut self, who: usize) {
        self.event_kind[who][1] = 0;
        self.object_mut(who).amount = 0;
        if !self.lights_out {
            self.room_dirty = true;
        }
    }

    fn collision(&mut self, what_kind: i64, who: usize) {
        match what_kind {
            CRASH_IT => {
                self.sound(snd::CRUNCH);
                self.crash();
            }
            LIFT_IT => {
                if !self.air_out {
                    self.lift_amount = -7;
                }
            }
            DROP_IT => {
                if !self.air_out {
                    self.lift_amount = 7;
                }
            }
            MOVE_IT => {
                self.sound(snd::HEY);
                self.transport(self.event_kind[who][1] as i32);
                self.hot_rect = Rect::NULL;
            }
            TURN_IT_LEFT => {
                self.shift_amount = -7;
                if self.life_normal && self.glider.is_right {
                    self.life_normal = false;
                    self.glider.mode = TURN_RT2LF;
                }
            }
            TURN_IT_RIGHT => {
                self.shift_amount = 7;
                if self.life_normal && !self.glider.is_right {
                    self.life_normal = false;
                    self.glider.mode = TURN_LF2RT;
                }
            }
            BURN_IT => {
                self.lift_amount = -7;
                if self.life_normal && self.glider.dest.bottom as i64 > self.event_kind[who][1] {
                    self.sound(snd::YOW);
                    self.life_normal = false;
                    let ticks = self.ticks;
                    let g = &mut self.glider;
                    if g.is_right {
                        g.phase = 1;
                    }
                    g.mode = BURNING;
                    g.time_stamp = ticks + 300;
                    g.dest.top = g.dest.bottom - 36;
                }
            }
            AWARD_IT => {
                if self.event_kind[who][1] != 0 {
                    self.sound(snd::CLOCK);
                    self.start_score_rolling();
                    let pts = self.event_kind[who][1];
                    self.supp_score += pts;
                    self.float_the_score(pts);
                    self.take_prize(who);
                }
            }
            EXTRA_IT => {
                if self.event_kind[who][1] != 0 {
                    self.sound(snd::EXTRA);
                    self.start_score_rolling();
                    let pts = self.event_kind[who][1];
                    self.supp_score += pts;
                    self.float_the_score(pts);
                    self.take_prize(who);
                    self.mortals += 1;
                }
            }
            SPILL_IT => {
                let r = &mut self.event_rect[who];
                r.left = r.right;
                r.top = r.bottom - 5;
                // tells the cycle loop the grease has gone over
                self.event_kind[who][0] = IGNORE_IT;
                self.object_mut(who).is_on = false;
            }
            SLIDE_IT => {
                let g = &mut self.glider;
                let d = self.event_rect[who].top - g.touch.bottom + 1;
                g.dest.top += d;
                g.dest.bottom += d;
                g.touch.top += d;
                g.touch.bottom += d;
                self.sliding = true;
            }
            BAND_IT => {
                if self.event_kind[who][1] != 0 {
                    self.sound(snd::GET_BAND);
                    let n = self.event_kind[who][1];
                    self.glider.bands += n as i32;
                    self.float_the_score(n);
                    self.take_prize(who);
                }
            }
            ENERGIZE_IT => {
                if self.event_kind[who][1] != 0 {
                    self.sound(snd::ENERGIZE);
                    let n = self.event_kind[who][1];
                    self.glider.energy += n as i32;
                    self.float_the_score(n);
                    self.take_prize(who);
                }
            }
            TRICK_IT => {
                if self.event_kind[who][1] != 0 {
                    self.sound(snd::WHISTLE);
                    self.start_score_rolling();
                    let pts = self.event_kind[who][1];
                    self.supp_score += pts;
                    self.float_the_score(pts);
                    self.event_kind[who][1] = 0;
                }
            }
            LIGHT_IT => {
                if self.lights_out {
                    self.sound(snd::CLICK);
                    self.lights_out = false;
                    self.room_mut().condition = 0;
                    self.room_dirty = true;
                }
            }
            AIR_ON_IT => {
                if self.air_out {
                    self.sound(snd::BLOWER_ON);
                    self.air_out = false;
                }
            }
            ZAP_IT => {
                if self.event_kind[who][1] != 0 {
                    self.sound(snd::ZAP);
                    self.crash();
                }
            }
            TOGGLE_IT => {
                if self.event_kind[who][2] < self.ticks {
                    self.sound(snd::CLICK);
                    let t = self.event_kind[who][1];
                    if (1..=16).contains(&t) {
                        let t = t as usize;
                        let kind = self.object(t).kind;
                        let on = !self.object(t).is_on;
                        self.object_mut(t).is_on = on;
                        match kind {
                            LFT_FAN | RIT_FAN => {
                                if self.event_kind[t][0] == IGNORE_IT {
                                    self.sound(snd::BLOWER_ON);
                                    self.event_kind[t][0] = if kind == LFT_FAN { TURN_IT_LEFT } else { TURN_IT_RIGHT };
                                } else {
                                    self.event_kind[t][0] = IGNORE_IT;
                                }
                            }
                            SHREDR => {
                                self.event_kind[t][0] = if self.event_kind[t][0] == IGNORE_IT { SHRED_IT } else { IGNORE_IT };
                            }
                            _ => {}
                        }
                    }
                    self.event_kind[who][2] = self.ticks + 90;
                }
            }
            PLAY_IT => {
                if self.event_kind[who][1] < self.ticks {
                    self.sound(snd::GUITAR);
                    self.event_kind[who][1] = self.ticks + 120;
                }
            }
            ASCEND_IT => {
                self.life_normal = false;
                self.glider.mode = ASCENDING;
                self.up_a_flight(self.event_kind[who][1] as i32);
                self.hot_rect = Rect::NULL;
            }
            DESCEND_IT => {
                self.life_normal = false;
                self.glider.mode = DESCENDING;
                self.down_a_flight(self.event_kind[who][1] as i32);
                self.hot_rect = Rect::NULL;
            }
            SHRED_IT => {
                self.life_normal = false;
                let er = self.event_rect[who];
                let g = &mut self.glider;
                g.mode = SHREDDING;
                g.dest.top = er.bottom;
                g.dest.bottom = g.dest.top;
                g.dest.left = er.left + 12;
                g.dest.right = g.dest.left + 48;
                g.shado_dest.left = g.dest.left;
                g.shado_dest.right = g.dest.right;
                g.phase = 0;
            }
            STEAM_IT => {
                self.lift_amount = -7;
                self.shift_amount = -7;
            }
            _ => {}
        }
    }

    fn get_it_legal(&mut self, outside: i32) {
        let g = &mut self.glider;
        for r in [&mut g.dest, &mut g.shado_dest, &mut g.touch] {
            r.left += outside;
            r.right += outside;
        }
    }

    fn search_rects(&mut self) {
        self.hot_rect = self.glider.touch;
        let d = self.glider.dest;
        if self.right_is_open {
            if d.left > 500 {
                self.advance_a_room();
                return;
            }
        } else if d.right > 512 {
            self.get_it_legal(512 - d.right);
        }
        let d = self.glider.dest;
        if self.left_is_open {
            if d.right < 12 {
                self.retreat_a_room();
                return;
            }
        } else if d.left < 0 {
            self.get_it_legal(-d.left);
        }
        if self.glider.dest.bottom > K_FLOOR_LIMIT {
            self.bite_it();
        }
        self.shift_amount = 0;
        // the bound is taken once, as Pascal's for loop does
        let n = self.n_objects;
        for index in 1..=n {
            if self.event_rect[index].sect(&self.hot_rect) {
                self.collision(self.event_kind[index][0], index);
            }
        }
        for i in 0..self.animates.len() {
            if self.animates[i].dest.sect(&self.hot_rect) {
                self.bite_it();
                break;
            }
        }
    }

    fn handle_band(&mut self) {
        if self.band.whole.left > 512 || self.band.whole.left < -16 {
            self.band_borne = false;
        }
        let b = &mut self.band;
        b.phase += 1;
        if b.phase > 2 {
            b.phase = 0;
        }
        b.whole = b.dest;
        b.dest.left += b.velocity;
        b.dest.right += b.velocity;
        if b.velocity > 0 {
            b.whole.right += b.velocity;
        } else {
            b.whole.left += b.velocity;
        }
        let dest = b.dest;
        for i in 0..self.animates.len() {
            let a = self.animates[i];
            if a.vert_off != 12 && a.dest.sect(&dest) {
                self.sound(if a.kind == 2 { snd::POP_BALLOON } else { snd::POP_DART });
                let an = &mut self.animates[i];
                an.phase = -1;
                an.hori_off = 0;
                an.vert_off = 12;
                self.start_score_rolling();
                self.supp_score += 100 * (3 - a.kind as i64);
            }
        }
    }

    fn push_it(&mut self, how_fast: i32) {
        if self.glider.energy > 0 {
            self.glider.energy -= 1;
            self.sound(snd::PUSH);
            self.glider.for_vel = how_fast;
        }
    }

    fn fire_band(&mut self) {
        if self.glider.bands > 0 {
            self.sound(snd::FIRE_BAND);
            self.glider.bands -= 1;
            self.band_borne = true;
            let g = self.glider;
            let b = &mut self.band;
            b.dest.top = g.dest.top + 6;
            b.dest.bottom = b.dest.top + 7;
            if g.is_right {
                b.dest.left = g.dest.right;
                b.velocity = 16;
            } else {
                b.dest.left = g.dest.left - 16;
                b.velocity = -16;
            }
            b.dest.right = b.dest.left + 16;
            b.whole = b.dest;
        }
    }

    /// Facing the way it flies?
    fn heading_right(&self) -> bool {
        self.glider.is_right == self.glider.is_forward
    }

    fn keys_steer(&mut self, c: Controls) {
        let g = &mut self.glider;
        if c.left {
            g.is_forward = !g.is_right;
            g.src_num = if g.is_right { 1 } else { 2 };
            g.for_vel = -K_MAX_THRUST;
        } else if c.right {
            g.is_forward = g.is_right;
            g.src_num = if g.is_right { 0 } else { 3 };
            g.for_vel = K_MAX_THRUST;
        }
    }

    fn get_input(&mut self, c: Controls) {
        match self.method {
            Method::HoldKey => {
                if c.left || c.right {
                    self.keys_steer(c);
                } else {
                    let g = &mut self.glider;
                    g.is_forward = true;
                    g.src_num = if g.is_right { 0 } else { 2 };
                    g.for_vel = 0;
                }
                if c.band && !self.band_borne {
                    self.fire_band();
                }
                if c.energy {
                    let v = if self.heading_right() { 16 } else { -16 };
                    self.push_it(v);
                }
            }
            Method::DropKey => {
                self.glider.for_vel = if self.heading_right() { K_MAX_THRUST } else { -K_MAX_THRUST };
                self.keys_steer(c);
                if c.band && !self.band_borne {
                    self.fire_band();
                }
                if c.energy {
                    let v = if self.heading_right() { 16 } else { -16 };
                    self.push_it(v);
                }
            }
            Method::AbsMouse => {
                if c.band && !self.band_borne {
                    self.fire_band();
                }
                let x = c.mouse_x.unwrap_or(256);
                self.glider.for_vel = ((x - 256) / 32).clamp(-K_MAX_THRUST, K_MAX_THRUST);
                let back = self.glider.for_vel < 0;
                let v = if back { -16 } else { 16 };
                {
                    let g = &mut self.glider;
                    match (g.is_right, back) {
                        (true, true) => {
                            g.is_forward = false;
                            g.src_num = 1;
                        }
                        (true, false) => {
                            g.is_forward = true;
                            g.src_num = 0;
                        }
                        (false, true) => {
                            g.is_forward = true;
                            g.src_num = 2;
                        }
                        (false, false) => {
                            g.is_forward = false;
                            g.src_num = 3;
                        }
                    }
                }
                if c.energy {
                    self.push_it(v);
                }
                // buttonFires is off by default: the button pushes
                if c.button {
                    self.push_it(v);
                }
            }
        }
    }

    fn move_glider(&mut self) {
        if self.window_open {
            self.lift_amount += self.randomize(11) - 5;
            self.shift_amount += self.randomize(11) - 5;
        }
        self.lift_amount += 1;
        let g = &mut self.glider;
        if self.sliding {
            self.lift_amount = 0;
            self.sliding = false;
        } else {
            g.dest.top += self.lift_amount;
            g.dest.bottom += self.lift_amount;
            g.touch.top += self.lift_amount;
            g.touch.bottom += self.lift_amount;
        }
        g.for_vel += self.shift_amount;
        g.dest.left += g.for_vel;
        g.dest.right += g.for_vel;
        g.touch.left = g.dest.left + 10;
        g.touch.right = g.dest.right - 10;
        g.shado_dest.left = g.dest.left;
        g.shado_dest.right = g.dest.right;
    }

    fn move_animates(&mut self) {
        for i in 0..self.animates.len() {
            if self.animates[i].unseen {
                if self.ticks > self.animates[i].tick_stamp {
                    self.reset_animate(-1, i);
                    self.animates[i].unseen = false;
                }
                continue;
            }
            let ticks = self.ticks;
            let a = &mut self.animates[i];
            if a.phase > -1 {
                a.phase += 1;
                if a.phase > 7 {
                    a.phase = 0;
                }
            }
            let mut whole = a.dest;
            match a.kind {
                0 | 1 => {
                    a.dest.offset(a.hori_off, a.vert_off);
                    whole.left += a.hori_off;
                    whole.bottom += a.vert_off;
                }
                2 => {
                    a.dest.top += a.vert_off;
                    a.dest.bottom += a.vert_off;
                    if a.vert_off > 0 {
                        whole.bottom += a.vert_off;
                    } else {
                        whole.top += a.vert_off;
                    }
                }
                _ => {}
            }
            if !whole.sect(&WHOLE_AREA) {
                a.tick_stamp = ticks + a.delay;
                a.dest = Rect::NULL;
                a.unseen = true;
            }
        }
    }

    fn move_cycle_obs(&mut self) {
        for i in 0..self.cycles.len() {
            let c = self.cycles[i];
            let t = c.tied_to;
            match c.kind {
                CANDLE => {
                    let c = &mut self.cycles[i];
                    c.position += 1;
                    if c.position > 50 {
                        c.position = 48;
                    }
                }
                GREASE => {
                    if self.event_kind[t][0] == IGNORE_IT {
                        self.sound(snd::GREASE_FALL);
                        self.event_kind[t][0] = SLIDE_IT;
                        self.cycles[i].reset = 1;
                    } else if c.reset != 0 && c.reset != 999 {
                        let c = &mut self.cycles[i];
                        c.reset += 1;
                        if c.reset > 4 {
                            c.velocity += 1;
                            self.event_rect[t].right += 1;
                            if c.velocity as i64 > c.position {
                                c.reset = 999;
                            }
                        } else {
                            c.accel = if c.reset < 3 { 58 } else { 59 };
                        }
                    }
                }
                OUTLET => {
                    let ticks = self.ticks;
                    let flip = self.binary_flip;
                    if c.accel == 25 {
                        if ticks > c.position {
                            let c = &mut self.cycles[i];
                            c.accel = flip + 51;
                            c.position = ticks;
                            self.event_kind[t][1] = 1;
                        }
                    } else if ticks > c.position + 60 {
                        let c = &mut self.cycles[i];
                        c.accel = 25;
                        c.position = ticks + c.reset;
                        self.event_kind[t][1] = 0;
                    } else {
                        self.sound(snd::ZAP);
                        self.cycles[i].accel = flip + 51;
                    }
                }
                DRIP => {
                    if c.phase < 57 {
                        if self.binary_flip != 0 {
                            let c = &mut self.cycles[i];
                            c.phase += 1;
                            if c.phase == 57 {
                                c.position += 160;
                            }
                        }
                    } else {
                        let mut c = c;
                        c.velocity += c.accel;
                        c.position += c.velocity as i64;
                        if c.position > c.reset {
                            self.sound(snd::DRIP);
                            c.velocity = 0;
                            c.position = c.hold.bottom as i64 * 32;
                            self.event_rect[t] = c.hold;
                            c.phase = 53;
                        }
                        let er = &mut self.event_rect[t];
                        c.whole = *er;
                        let diff = ((c.position >> 5) as i16 as i32) - er.bottom;
                        er.bottom += diff;
                        er.top += diff;
                        c.whole.bottom += diff;
                        self.cycles[i] = c;
                    }
                }
                TOASTR => {
                    let mut c = c;
                    if c.phase != 0 {
                        if self.binary_flip == 0 {
                            c.phase += 1;
                            if c.phase > 65 {
                                c.phase = 60;
                            }
                        }
                        c.velocity += c.accel;
                        c.position += c.velocity as i64;
                        if c.position > c.reset {
                            self.sound(snd::TOAST_LAND);
                            c.velocity = -c.velocity;
                            c.position = c.reset;
                            c.reset = self.ticks + self.object(t).extra as i64;
                            c.phase = 0;
                        }
                        self.bob(&mut c, t);
                    } else if self.ticks > c.reset {
                        self.sound(snd::TOAST_LAUNCH);
                        c.reset = c.position;
                        c.phase = 60;
                    }
                    self.cycles[i] = c;
                }
                BALL => {
                    let mut c = c;
                    c.velocity += c.accel;
                    c.position += c.velocity as i64;
                    if c.position > c.reset {
                        self.sound(snd::BOUNCE);
                        c.velocity = -c.velocity;
                        c.position = c.reset;
                    }
                    self.bob(&mut c, t);
                    self.cycles[i] = c;
                }
                FSH_BWL => {
                    let mut c = c;
                    if c.phase != 0 {
                        c.phase = if c.velocity > -16 && c.velocity < 16 {
                            69
                        } else if c.velocity < 0 {
                            66
                        } else {
                            68
                        };
                        c.velocity += c.accel;
                        c.position += c.velocity as i64;
                        if c.position > c.reset {
                            self.sound(snd::DRIP);
                            c.velocity = -c.velocity;
                            c.position = c.reset;
                            c.reset = self.ticks + self.object(t).extra as i64;
                            c.phase = 0;
                        }
                        self.bob(&mut c, t);
                    } else if self.ticks > c.reset {
                        self.sound(snd::DRIP);
                        c.reset = c.position;
                        c.phase = 66;
                    }
                    self.cycles[i] = c;
                }
                TEA_KTL => {
                    if self.ticks > c.reset {
                        self.event_kind[t][0] = STEAM_IT;
                        if c.phase == 0 {
                            self.sound(snd::TEA_KETTLE);
                        }
                        let ticks = self.ticks;
                        let c = &mut self.cycles[i];
                        c.phase += 1;
                        if c.phase > 10 {
                            c.phase = 0;
                            self.event_kind[t][0] = IGNORE_IT;
                            c.reset = ticks + c.position;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// The bottom of a bouncing thing follows `position` (in 32nds).
    fn bob(&mut self, c: &mut Cycle, t: usize) {
        let er = &mut self.event_rect[t];
        c.whole = *er;
        let diff = ((c.position >> 5) as i16 as i32) - er.bottom;
        er.bottom += diff;
        er.top += diff;
        if diff > 0 {
            c.whole.bottom += diff;
        } else {
            c.whole.top += diff;
        }
    }

    fn monitor_storm(&mut self) {
        let l = &mut self.lightning;
        if l.what_phase == 0 {
            if self.ticks > l.what_time {
                l.what_phase = 1;
            }
        } else if l.what_phase > 3 {
            l.what_phase = 0;
            let b = self.randomize(3) as usize + 1;
            let t = self.randomize(300) as i64;
            self.lightning.which_bolt = b;
            self.lightning.what_time = self.ticks + t;
            self.sound(snd::THUNDER);
        } else {
            l.what_phase += 1;
        }
    }

    fn do_special(&mut self) {
        let mode = self.glider.mode;
        match mode {
            FADING_IN => {
                if self.glider.phase == 1 {
                    self.sound(snd::BEAM_IN);
                }
                let g = &mut self.glider;
                g.phase += 1;
                if g.phase > 16 {
                    g.src_num = if g.is_right { 0 } else { 2 };
                    g.phase = 0;
                    g.mode = NORMAL;
                    self.life_normal = true;
                }
            }
            FADING_OUT => {
                let g = &mut self.glider;
                g.phase += 1;
                g.src_num = if g.is_right { 0 } else { 2 };
                if g.phase > 16 {
                    self.off_a_mortal();
                }
            }
            TURN_RT2LF | TURN_LF2RT => {
                let g = &mut self.glider;
                g.phase += 1;
                if g.phase > 11 {
                    g.phase = 0;
                    g.mode = NORMAL;
                    g.is_right = mode == TURN_LF2RT;
                    self.life_normal = true;
                    self.shift_amount = 0;
                }
                self.search_rects();
                let s = self.shift_amount;
                let g = &mut self.glider;
                for r in [&mut g.dest, &mut g.touch, &mut g.shado_dest] {
                    r.left += s;
                    r.right += s;
                }
            }
            BURNING => {
                let g = &mut self.glider;
                g.phase = 1 - g.phase;
                if g.is_right {
                    g.src_num = 24 + g.phase;
                    g.for_vel = 1;
                } else {
                    g.src_num = 26 + g.phase;
                    g.for_vel = -1;
                }
                self.search_rects();
                self.move_glider();
                if self.ticks > self.glider.time_stamp {
                    self.sound(snd::CRUNCH);
                    let g = &mut self.glider;
                    g.mode = FADING_OUT;
                    g.phase = 0;
                    g.dest.top = g.dest.bottom - 20;
                    g.src_num = if g.is_forward { 0 } else { 2 };
                }
            }
            ASCENDING | DESCENDING => {
                let (dx, dy) = if mode == ASCENDING { (-2, -6) } else { (2, 6) };
                let g = &mut self.glider;
                g.dest.left += dx;
                g.dest.right = g.dest.left + 48;
                g.dest.top += dy;
                g.dest.bottom = g.dest.top + 20;
                g.shado_dest.left = g.dest.left;
                g.shado_dest.right = g.dest.right;
                g.touch = g.dest.inset_by(10, 5);
                let done = if mode == ASCENDING { g.dest.top < 220 } else { g.dest.top > 120 };
                if done {
                    g.phase = 0;
                    g.mode = NORMAL;
                    self.life_normal = true;
                }
            }
            SHREDDING => {
                if self.glider.phase == 0 {
                    self.sound(snd::SHRED);
                    let g = &mut self.glider;
                    g.dest.bottom += 1;
                    if g.dest.bottom >= g.dest.top + 36 {
                        g.phase = 1;
                    }
                } else {
                    let g = &mut self.glider;
                    g.dest.top += 8;
                    g.dest.bottom += 8;
                    if g.dest.top > 342 {
                        g.phase = 0;
                        self.off_a_mortal();
                    }
                }
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------
    // G-PlayUtils.p

    fn reset_glider(&mut self) {
        let entered_left = self.entered_left;
        let g = &mut self.glider;
        if g.mode != BURNING {
            g.time_stamp = 0;
            g.mode = FADING_IN;
            g.phase = 0;
            g.is_forward = true;
            g.src_num = if g.is_right { 0 } else { 2 };
        }
        g.dest = if entered_left { Rect::new(0, 40, 48, 60) } else { Rect::new(464, 40, 512, 60) };
        g.old = g.dest;
        g.shado_dest = g.dest;
        g.shado_dest.top = K_FLOOR_VERT;
        g.shado_dest.bottom = K_FLOOR_VERT + 11;
        g.touch = g.dest.inset_by(10, 5);
        self.life_normal = false;
    }

    fn reset_animate(&mut self, type_is: i32, i: usize) {
        if type_is != -1 {
            let delay = self.room().animate_delay as i64;
            let a = &mut self.animates[i];
            a.delay = delay;
            a.kind = type_is;
            a.unseen = true;
        }
        let kind = self.animates[i].kind;
        match kind {
            0 => {
                let top = self.randomize(150);
                let a = &mut self.animates[i];
                a.dest = Rect::new(512, top, 576, top + 22);
                a.phase = 0;
                a.hori_off = -8;
                a.vert_off = 1;
            }
            1 => {
                let left = self.randomize(256) + 256;
                let phase = self.randomize(8);
                let a = &mut self.animates[i];
                a.dest = Rect::new(left, -32, left + 32, 0);
                a.phase = phase;
                a.hori_off = -4;
                a.vert_off = 2;
            }
            2 => {
                let left = self.randomize(400) + 50;
                let phase = self.randomize(8);
                let a = &mut self.animates[i];
                a.dest = Rect::new(left, 342, left + 32, 374);
                a.phase = phase;
                a.hori_off = 0;
                a.vert_off = -3;
            }
            _ => {}
        }
        let lo = self.animates[i].delay as i16 as i32;
        let r = self.randomize(lo + 120) as i64;
        self.animates[i].tick_stamp = r + self.ticks + 60;
    }

    fn extract_event_rects(&mut self) {
        self.window_rgn.clear();
        self.mirror_rgn = None;
        self.toast_rgn = None;
        self.window_open = false;
        let cond = self.room().condition;
        self.lights_out = cond == 2;
        self.air_out = cond == 1;
        self.cycles.clear();
        let n = self.room().n_objects as usize;
        self.n_objects = n;
        for index in 1..=n {
            let o = self.object(index);
            let b = o.bound;
            let mid = (b.left + b.right) / 2;
            self.event_rect[index] = match o.kind {
                TABLE..=OBS_RCT | CLOCK | PAPER | BNS_RCT..=RBR_BND | LIT_SWT | THERMO | PWR_SWT | BASKET | MAC_TSH => b,
                FLR_VNT => Rect::new(mid - 8, o.amount, mid + 8, K_FLOOR_VERT),
                CEL_VNT => Rect::new(mid - 8, K_CEILING_VERT, mid + 8, o.amount),
                CEL_DCT => {
                    if o.is_on {
                        Rect::new(mid - 8, K_CEILING_VERT, mid + 8, o.amount)
                    } else {
                        Rect::new(b.left, b.top, b.right, b.top + 8)
                    }
                }
                CANDLE => {
                    let position = self.randomize(3) as i64 + 48;
                    self.cycles.push(Cycle { whole: Rect::new(b.left + 5, b.top - 12, b.left + 21, b.top), kind: CANDLE, position, ..Cycle::default() });
                    Rect::new(mid - 12, o.amount, mid + 4, b.top)
                }
                LFT_FAN => Rect::new(o.amount, b.top + 10, b.left, b.top + 30),
                RIT_FAN => Rect::new(b.right, b.top + 10, o.amount, b.top + 30),
                GREASE => {
                    if o.is_on {
                        self.cycles.push(Cycle { kind: GREASE, tied_to: index, whole: b, reset: 0, accel: GREASE, velocity: b.right, position: o.amount as i64, ..Cycle::default() });
                        b
                    } else {
                        self.cycles.push(Cycle { kind: GREASE, tied_to: index, whole: b, reset: 999, accel: 59, velocity: o.amount, position: o.amount as i64, ..Cycle::default() });
                        Rect::new(b.right, b.bottom - 5, o.amount, b.bottom)
                    }
                }
                SHREDR => Rect::new(b.left, b.top - 4, b.right, b.bottom - 12),
                OUTLET => {
                    self.event_kind[index][1] = 0;
                    self.cycles.push(Cycle { kind: OUTLET, tied_to: index, accel: 25, whole: b, reset: o.amount as i64, position: self.ticks, ..Cycle::default() });
                    b
                }
                GUITAR => Rect::new(b.left + 36, b.top + 24, b.left + 38, b.bottom - 56),
                BALL => {
                    let reset = b.bottom as i64 * 32;
                    let velocity = launch(o.amount, reset);
                    self.cycles.push(Cycle { kind: BALL, tied_to: index, whole: b, reset, accel: 8, position: reset, velocity, ..Cycle::default() });
                    b
                }
                DRIP => {
                    self.cycles.push(Cycle {
                        kind: DRIP,
                        tied_to: index,
                        hold: b,
                        whole: b,
                        reset: o.amount as i64 * 32,
                        accel: 12,
                        position: b.bottom as i64 * 32,
                        velocity: 0,
                        phase: 53,
                    });
                    b
                }
                TOASTR => {
                    let (x, y) = (b.left + 3, b.bottom - 3);
                    let er = Rect::new(x, y, x + 32, y + 31);
                    self.toast_rgn = Some(Rect::new(x, o.amount, x + 32, b.top));
                    let reset = y as i64 * 32;
                    let velocity = launch(o.amount, reset);
                    self.cycles.push(Cycle { phase: 60, kind: TOASTR, tied_to: index, whole: er, reset, accel: 8, position: reset, velocity, ..Cycle::default() });
                    er
                }
                FSH_BWL => {
                    let (x, y) = (b.left + 8, b.top + 24);
                    let er = Rect::new(x, y, x + 16, y + 16);
                    let reset = y as i64 * 32;
                    let velocity = launch(o.amount, reset);
                    self.cycles.push(Cycle { phase: 69, kind: FSH_BWL, tied_to: index, whole: er, reset, accel: 8, position: reset, velocity, ..Cycle::default() });
                    er
                }
                TEA_KTL => {
                    let mut er = Rect::new(b.left - 128, b.top - 128, b.left, b.top);
                    er.top = er.top.max(K_CEILING_VERT);
                    er.left = er.left.max(0);
                    let position = o.amount as i64;
                    let reset = position + self.randomize(120) as i64;
                    self.cycles.push(Cycle { phase: 0, kind: TEA_KTL, tied_to: index, whole: Rect::default(), position, reset, ..Cycle::default() });
                    er
                }
                WINDOW => {
                    self.n_objects -= 1;
                    self.frame_window(o);
                    self.event_rect[index]
                }
                PAINTG => {
                    self.n_objects -= 1;
                    self.event_rect[index]
                }
                MIRROR => {
                    self.n_objects -= 1;
                    self.mirror_rgn = Some(b.inset_by(5, 5));
                    self.event_rect[index]
                }
                UP_STAR => Rect::new(b.left + 32, b.top, b.right - 32, b.top + 8),
                DN_STAR => Rect::new(b.left + 32, b.bottom - 8, b.right - 32, b.bottom),
                _ => Rect::NULL,
            };
            self.event_kind[index][1] = 0;
            self.event_kind[index][0] = match o.kind {
                TABLE | SHELF | BOOKS | CABNET | OBS_RCT | DRIP | TOASTR | BALL | FSH_BWL | BASKET | MAC_TSH => CRASH_IT,
                EXT_RCT => {
                    self.event_kind[index][1] = o.amount as i64;
                    MOVE_IT
                }
                FLR_VNT => LIFT_IT,
                CEL_VNT => DROP_IT,
                CEL_DCT => {
                    if o.is_on {
                        DROP_IT
                    } else {
                        self.event_kind[index][1] = o.extra as i64;
                        MOVE_IT
                    }
                }
                CANDLE => {
                    self.event_kind[index][1] = b.bottom as i64 - 48;
                    BURN_IT
                }
                LFT_FAN => {
                    if o.is_on {
                        TURN_IT_LEFT
                    } else {
                        IGNORE_IT
                    }
                }
                RIT_FAN => {
                    if o.is_on {
                        TURN_IT_RIGHT
                    } else {
                        IGNORE_IT
                    }
                }
                CLOCK | PAPER | BNS_RCT | BATTRY | RBR_BND => {
                    self.event_kind[index][1] = o.amount as i64;
                    if o.amount < 1 {
                        self.event_rect[index] = Rect::NULL;
                    }
                    match o.kind {
                        CLOCK => AWARD_IT,
                        PAPER => EXTRA_IT,
                        BNS_RCT => TRICK_IT,
                        BATTRY => ENERGIZE_IT,
                        _ => BAND_IT,
                    }
                }
                GREASE => {
                    if o.is_on {
                        SPILL_IT
                    } else {
                        SLIDE_IT
                    }
                }
                LIT_SWT => LIGHT_IT,
                OUTLET => ZAP_IT,
                THERMO => AIR_ON_IT,
                SHREDR => {
                    if o.is_on {
                        SHRED_IT
                    } else {
                        IGNORE_IT
                    }
                }
                PWR_SWT => {
                    self.event_kind[index][1] = o.amount as i64;
                    self.event_kind[index][2] = self.ticks;
                    TOGGLE_IT
                }
                GUITAR => PLAY_IT,
                UP_STAR => {
                    self.event_kind[index][1] = o.amount as i64;
                    ASCEND_IT
                }
                DN_STAR => {
                    self.event_kind[index][1] = o.amount as i64;
                    DESCEND_IT
                }
                _ => IGNORE_IT,
            };
        }
    }

    fn frame_window(&mut self, o: Object) {
        let b = o.bound;
        let (top_r, bottom_r);
        if o.is_on {
            let mut t = b.inset_by(8, 16);
            t.top = (b.bottom + b.top) / 2 - 2;
            t.offset(0, 26 - (b.bottom - b.top) / 2);
            top_r = t.inset_by(10, 10);
            let mut bt = b.inset_by(8, 16);
            bt.top = (b.bottom + b.top) / 2 + 10;
            bottom_r = bt;
            self.window_open = true;
        } else {
            let mut t = b.inset_by(8, 16);
            t.bottom = (b.bottom + b.top) / 2 + 2;
            top_r = t.inset_by(10, 10);
            let mut bt = b.inset_by(8, 16);
            bt.top = (b.bottom + b.top) / 2 - 2;
            bottom_r = bt.inset_by(10, 10);
        }
        self.window_rgn = vec![top_r, bottom_r];
        let r = top_r.union(&bottom_r);
        let step = (r.bottom - r.top) / 7;
        for i in 1..=3 {
            let x0 = r.left + self.randomize(r.right - r.left);
            self.lightning.bolts[i][0] = [x0, r.top];
            for joint in 1..8 {
                // the Pascal tries until the joint lands inside the window
                for _ in 0..10_000 {
                    let x = self.lightning.bolts[i][joint - 1][0] + self.randomize(step * 4) - step * 2;
                    self.lightning.bolts[i][joint] = [x, step * joint as i32 + r.top];
                    if x > r.left && x < r.right {
                        break;
                    }
                }
            }
        }
        self.lightning.which_bolt = self.randomize(3) as usize + 1;
        self.lightning.what_time = self.ticks + 120 + self.randomize(120) as i64;
        self.lightning.what_phase = 0;
    }

    fn ready_room(&mut self) {
        let room = self.room().clone();
        self.right_is_open = room.right_open;
        self.left_is_open = room.left_open;
        self.animates.clear();
        if room.animate_number > 0 {
            for i in 0..room.animate_number.min(16) as usize {
                self.animates.push(Animate::default());
                self.reset_animate(room.animate_kind, i);
            }
        }
        self.band_borne = false;
        self.float_points = None;
        self.extract_event_rects();
        self.ticks += self.ready_room_ticks;
        self.sound(snd::MUSIC);
        self.room_dirty = true;
    }

    // ---------------------------------------------------------------
    // G-PlaySetUp.p

    fn calc_room_score(&mut self) {
        self.room_score = 0;
        for index in 1..=MAX_ROOMS {
            if self.room_visits[index] {
                self.room_score += 500 * (index as i64 / 10 + 1) + (self.rooms_passed as i64 / 41) * 2000;
            }
        }
    }

    fn check_for_time_bonus(&mut self) {
        let mut bonus = 0;
        if self.loops_thru_room < K_BONUS_TIME_TO_BEAT {
            bonus = (K_BONUS_TIME_TO_BEAT - self.loops_thru_room) * (self.room_at as i64 / 5 + 1);
        }
        self.supp_score += bonus;
        self.loops_thru_room = 0;
        self.fx.push(Fx::TimeBonus(bonus));
        self.ticks += TIME_BONUS_TICKS;
    }

    fn set_room_state(&mut self) {
        if self.didnt_exit_entrance && (1..=MAX_ROOMS as i32).contains(&self.room_at) {
            self.room_visits[self.room_at as usize] = true;
        }
        self.bass_loop = 0;
        self.play_bass_time = K_BONUS_TIME_TO_BEAT / K_BASS_FRACT + K_MIN_BASS_LOOP;
        self.start_score_rolling();
        let old = self.room_score;
        self.calc_room_score();
        if old < self.room_score && self.didnt_exit_entrance {
            self.check_for_time_bonus();
        }
        self.loops_thru_room = 0;
        for i in 1..=self.n_objects {
            match self.event_kind[i][0] {
                AWARD_IT | EXTRA_IT | ENERGIZE_IT | BAND_IT | TRICK_IT => {
                    let a = self.event_kind[i][1] as i32;
                    self.object_mut(i).amount = a;
                }
                LIGHT_IT | AIR_ON_IT => {
                    if !self.lights_out && !self.air_out {
                        self.room_mut().condition = 0;
                    }
                }
                _ => {}
            }
        }
    }

    /// Straight into a room, the glider at the top as a duct leaves it: for
    /// looking at rooms (`--shot --room`), not part of the game.
    pub fn warp(&mut self, room: i32) {
        self.room_at = room.clamp(1, self.house.n_rooms.max(1));
        self.ready_room();
        self.place_glider(Rect::new(232, K_CEILING_VERT + 20, 280, K_CEILING_VERT + 40));
        self.fx.clear();
    }

    fn place_glider(&mut self, dest: Rect) {
        let g = &mut self.glider;
        g.dest = dest;
        g.old = dest;
        g.shado_dest.left = dest.left;
        g.shado_dest.right = dest.right;
        g.touch = dest.inset_by(10, 5);
    }

    /// The house is over: by finishing it, or leaving through room 0.
    fn house_done(&mut self) {
        self.room_at = 0;
        self.fx.push(Fx::Closing);
        self.wrap_it_up();
    }

    fn wrap_it_up(&mut self) {
        self.playing = false;
        self.window_rgn.clear();
        self.mirror_rgn = None;
        self.toast_rgn = None;
        self.fx.push(Fx::GameOver);
    }

    /// AdvanceAFile: on into the next house of the sequence. False when it
    /// cannot be found (the Pascal ends the game then).
    fn advance_a_file(&mut self) -> bool {
        let next = self.house.next_file.clone();
        let Some(h) = self.lib.load(&next) else {
            log::warn!("next house {:?} not found", crate::mac::mac_roman(&next));
            return false;
        };
        self.house = h;
        self.house_name = next;
        self.calc_room_score();
        self.supp_score += self.room_score;
        self.room_visits = [false; MAX_ROOMS + 1];
        self.room_score = 0;
        true
    }

    fn off_a_mortal(&mut self) {
        self.mortals -= 1;
        if self.mortals < 1 {
            self.wrap_it_up();
            return;
        }
        self.reset_glider();
    }

    fn advance_a_room(&mut self) {
        self.didnt_exit_entrance = self.side_you_cant_exit != RIGHT_OF_ROOM;
        self.set_room_state();
        self.entered_left = true;
        self.side_you_cant_exit = LEFT_OF_ROOM;
        self.room_at += 1;
        self.rooms_passed += 1;
        if self.room_at > self.house.n_rooms {
            if !names_a_file(&self.house.next_file.clone()) || !self.advance_a_file() {
                self.rooms_passed -= 1;
                self.house_done();
                return;
            }
            self.room_at = 1;
        }
        self.ready_room();
        self.place_glider(Rect::new(0, self.glider.dest.top, 48, self.glider.dest.bottom));
        self.fx.push(Fx::ScrollForward);
        self.ticks += SCROLL_SIDE_TICKS;
    }

    fn retreat_a_room(&mut self) {
        self.didnt_exit_entrance = self.side_you_cant_exit != LEFT_OF_ROOM;
        self.set_room_state();
        self.entered_left = false;
        self.side_you_cant_exit = RIGHT_OF_ROOM;
        self.room_at -= 1;
        self.rooms_passed -= 1;
        if self.room_at < 1 {
            self.rooms_passed -= 1;
            self.house_done();
            return;
        }
        self.ready_room();
        self.place_glider(Rect::new(464, self.glider.dest.top, 512, self.glider.dest.bottom));
        self.fx.push(Fx::ScrollBackward);
        self.ticks += SCROLL_SIDE_TICKS;
    }

    /// Where a flight of stairs leads: the room, in this house or (past its
    /// end) the next. None ends the game.
    fn flight_to(&mut self, what_room: i32) -> Option<i32> {
        let add = what_room - self.room_at;
        self.room_at = what_room;
        self.rooms_passed += add;
        if self.room_at < 0 || self.room_at > self.house.n_rooms {
            if !names_a_file(&self.house.next_file.clone()) || self.room_at < 0 || !self.advance_a_file() {
                return None;
            }
            self.room_at -= 40;
        } else if self.room_at == 0 {
            return None;
        }
        Some(self.room_at)
    }

    fn up_a_flight(&mut self, what_room: i32) {
        self.didnt_exit_entrance = self.side_you_cant_exit != TOP_OF_ROOM;
        self.set_room_state();
        self.side_you_cant_exit = BOTTOM_OF_ROOM;
        if self.flight_to(what_room).is_none() {
            self.rooms_passed -= 1;
            self.house_done();
            return;
        }
        self.ready_room();
        // ScrollUp: in at the foot of the stairs down
        let mut left = 232;
        for o in &self.room().objects {
            if o.kind == DN_STAR {
                left = o.bound.left + 64;
            }
        }
        self.place_glider(Rect::new(left, K_FLOOR_VERT - 20, left + 48, K_FLOOR_VERT));
        self.fx.push(Fx::ScrollUp);
        self.ticks += SCROLL_UPDOWN_TICKS;
    }

    fn down_a_flight(&mut self, what_room: i32) {
        self.didnt_exit_entrance = self.side_you_cant_exit != BOTTOM_OF_ROOM;
        self.set_room_state();
        self.side_you_cant_exit = TOP_OF_ROOM;
        if self.flight_to(what_room).is_none() {
            // DownAFlight takes the room back off only when it ran past the house
            if self.room_at != 0 {
                self.rooms_passed -= 1;
            }
            self.house_done();
            return;
        }
        self.ready_room();
        // ScrollDown: in at the top of the stairs up
        let mut left = 232;
        for o in &self.room().objects {
            if o.kind == UP_STAR {
                left = o.bound.left + 64;
            }
        }
        self.place_glider(Rect::new(left, K_CEILING_VERT + 20, left + 48, K_CEILING_VERT + 40));
        self.fx.push(Fx::ScrollDown);
        self.ticks += SCROLL_UPDOWN_TICKS;
    }

    fn transport(&mut self, what_room: i32) {
        let new_room = what_room != self.room_at;
        if new_room {
            self.didnt_exit_entrance = true;
            self.set_room_state();
            self.side_you_cant_exit = WHO_CARES;
            let add = what_room - self.room_at;
            self.room_at = what_room;
            self.rooms_passed += add;
            if self.room_at < 0 || self.room_at > self.house.n_rooms {
                let local = self.room_at - self.house.n_rooms;
                if !names_a_file(&self.house.next_file.clone()) || self.room_at < 0 || !self.advance_a_file() {
                    self.house_done();
                    return;
                }
                self.room_at = local;
                if self.room_at > self.house.n_rooms {
                    self.house_done();
                    return;
                }
            } else if self.room_at == 0 {
                self.house_done();
                return;
            }
            self.ready_room();
        }
        let mut left = 232;
        for o in &self.room().objects {
            if o.kind == CEL_DCT && o.is_on {
                left = o.bound.left;
            }
        }
        self.place_glider(Rect::new(left, K_CEILING_VERT, left + 48, K_CEILING_VERT + 20));
        if new_room {
            self.fx.push(Fx::Dissolve);
            self.ticks += DISSOLVE_TICKS;
        }
    }
}

/// The speed that throws a ball, toast or fish from `reset` (a bottom in
/// 32nds of a pixel) up to `amount`: the Pascal's repeat loop.
fn launch(amount: i32, reset: i64) -> i32 {
    let accel = 8;
    let mut temp = (amount as i64 + 32) * 32;
    let mut velocity: i32 = 0;
    loop {
        velocity += accel;
        temp += velocity as i64;
        if temp > reset {
            break;
        }
    }
    -velocity
}
