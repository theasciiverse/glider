//! Keyboard, mouse and gamepad, turned into Glider's controls.
//!
//! - Left and right: arrows, `,` and `.` (Glider's own keys), A and D;
//!   the gamepad's d-pad or stick
//! - Battery boost: Up, Enter (Glider's Return), W; pad B
//! - Rubber band: Space (Glider's), Down, S; pad A
//! - Mouse: the pointer's place left or right of the room's middle steers,
//!   as Glider's mouse control did; the button boosts
//!
//! With the kitty keyboard protocol presses and releases are exact.
//! Without it the terminal only sends key repeats, so a key counts as held
//! while they keep arriving.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

/// Without key releases, a press counts as held this long (until the
/// terminal's repeats arrive), then as long as they keep coming.
const PRE_REPEAT: Duration = Duration::from_millis(550);
const REPEAT_GAP: Duration = Duration::from_millis(100);

const LEFT: usize = 0;
const RIGHT: usize = 1;
const ENERGY: usize = 2;
const BAND: usize = 3;
const KEYS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    Up,
    Down,
    Confirm,
    Back,
    Pause,
    Quit,
    Sound,
    Music,
    Help,
    Houses,
    Scores,
    View,
    Exit,
    Any,
    Char(char),
    Backspace,
    /// A left or right key: the player is on the keyboard.
    Steer,
    /// The pad steered: hold control, whatever the terminal reports, since
    /// a pad's presses and releases are exact.
    PadSteer,
}

#[derive(Clone, Copy, Default)]
struct Key {
    down: bool,
    latched: bool,
    last: Option<Instant>,
    repeats: u32,
}

pub struct Input {
    pub exact: bool,
    keys: [Key; KEYS],
    pub cmds: Vec<Cmd>,
    pub mouse_x: Option<i32>,
    pub mouse_moved: bool,
    pub button: bool,
    button_latched: bool,
    #[cfg(feature = "gamepad")]
    gilrs: Option<gilrs::Gilrs>,
    pub pad_name: Option<String>,
    pad: [bool; KEYS],
    /// Where the stick last pointed: -1 left, 0 the middle, 1 right.
    stick: i32,
}

impl Input {
    pub fn new(exact: bool) -> Input {
        Input {
            exact,
            keys: [Key::default(); KEYS],
            cmds: Vec::new(),
            mouse_x: None,
            mouse_moved: false,
            button: false,
            button_latched: false,
            #[cfg(feature = "gamepad")]
            gilrs: None,
            pad_name: None,
            pad: [false; KEYS],
            stick: 0,
        }
    }

    pub fn on_key(&mut self, k: KeyEvent, now: Instant) {
        let press = k.kind != KeyEventKind::Release;
        let fresh = k.kind == KeyEventKind::Press;
        if fresh && k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c' | 'C')) {
            self.cmds.push(Cmd::Exit);
            return;
        }
        let which = match k.code {
            KeyCode::Left | KeyCode::Char(',' | '<' | 'a' | 'A') => Some(LEFT),
            KeyCode::Right | KeyCode::Char('.' | '>' | 'd' | 'D') => Some(RIGHT),
            KeyCode::Up | KeyCode::Enter | KeyCode::Char('w' | 'W') => Some(ENERGY),
            KeyCode::Down | KeyCode::Char(' ' | 's' | 'S') => Some(BAND),
            _ => None,
        };
        if let Some(d) = which {
            let key = &mut self.keys[d];
            if self.exact {
                if press && !key.down {
                    key.latched = true;
                }
                key.down = press;
            } else {
                if held_by_repeat(key, now) {
                    key.repeats += 1;
                } else {
                    key.repeats = 0;
                    key.latched = true;
                }
                key.last = Some(now);
            }
            if fresh && (d == LEFT || d == RIGHT) {
                self.cmds.push(Cmd::Steer);
            }
        }
        if !press {
            return;
        }
        match k.code {
            KeyCode::Up => self.cmds.push(Cmd::Up),
            KeyCode::Down => self.cmds.push(Cmd::Down),
            KeyCode::Backspace => self.cmds.push(Cmd::Backspace),
            _ => {}
        }
        if !fresh {
            return;
        }
        if !matches!(k.code, KeyCode::Modifier(_)) {
            self.cmds.push(Cmd::Any);
        }
        if let KeyCode::Char(c) = k.code {
            self.cmds.push(Cmd::Char(c));
        }
        match k.code {
            KeyCode::Enter => self.cmds.push(Cmd::Confirm),
            KeyCode::Esc => self.cmds.push(Cmd::Back),
            KeyCode::Tab | KeyCode::Char('p' | 'P') => self.cmds.push(Cmd::Pause),
            KeyCode::Char('q' | 'Q') => self.cmds.push(Cmd::Quit),
            KeyCode::Char('m' | 'M') => self.cmds.push(Cmd::Sound),
            KeyCode::Char('b' | 'B') => self.cmds.push(Cmd::Music),
            KeyCode::Char('?' | 'h' | 'H') | KeyCode::F(1) => self.cmds.push(Cmd::Help),
            KeyCode::Char('l' | 'L') => self.cmds.push(Cmd::Houses),
            KeyCode::Char('t' | 'T') => self.cmds.push(Cmd::Scores),
            KeyCode::Char('v' | 'V') | KeyCode::F(2) => self.cmds.push(Cmd::View),
            _ => {}
        }
    }

    pub fn mouse_move(&mut self, x: Option<i32>) {
        if x.is_some() && x != self.mouse_x {
            self.mouse_moved = true;
        }
        self.mouse_x = x.or(self.mouse_x);
    }

    pub fn mouse_down(&mut self) {
        self.button = true;
        self.button_latched = true;
        self.cmds.push(Cmd::Any);
    }

    pub fn mouse_up(&mut self) {
        self.button = false;
    }

    /// The window lost focus: releases will not arrive, so let go.
    pub fn release_all(&mut self) {
        for k in &mut self.keys {
            k.down = false;
            k.last = None;
        }
        self.button = false;
    }

    /// Forget presses made before now (when a game starts).
    pub fn clear(&mut self) {
        self.release_all();
        for k in &mut self.keys {
            k.latched = false;
        }
        self.button_latched = false;
        self.mouse_moved = false;
        self.cmds.clear();
    }

    pub fn open_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        match gilrs::Gilrs::new() {
            Ok(g) => {
                self.pad_name = first_pad(&g);
                log::info!("gamepad: {:?}", self.pad_name);
                self.gilrs = Some(g);
            }
            Err(e) => log::warn!("gamepad support unavailable: {e}"),
        }
    }

    pub fn poll_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        {
            use gilrs::{Axis, Button, EventType};
            let Some(g) = &mut self.gilrs else { return };
            while let Some(ev) = g.next_event() {
                if matches!(ev.event, EventType::Connected | EventType::Disconnected) {
                    self.pad_name = first_pad(g);
                    log::info!("gamepad: {:?}", self.pad_name);
                }
                if !is_pad(&g.gamepad(ev.id)) {
                    continue;
                }
                if ev.event == EventType::Disconnected {
                    self.pad = [false; KEYS];
                    self.stick = 0;
                    continue;
                }
                match ev.event {
                    EventType::ButtonPressed(b, _) => {
                        log::debug!("pad press {b:?}");
                        self.cmds.push(Cmd::Any);
                        match b {
                            Button::South => self.cmds.push(Cmd::Confirm),
                            Button::Start => self.cmds.push(Cmd::Pause),
                            Button::Select => self.cmds.push(Cmd::Back),
                            Button::DPadUp => self.cmds.push(Cmd::Up),
                            Button::DPadDown => self.cmds.push(Cmd::Down),
                            Button::DPadLeft | Button::DPadRight => self.cmds.push(Cmd::PadSteer),
                            _ => {}
                        }
                    }
                    _ => {}
                }
                let pad = g.gamepad(ev.id);
                let x = pad.axis_data(Axis::LeftStickX).map_or(0.0, |d| d.value());
                // pushing the stick over steers as the d-pad does
                let stick = if x < -0.4 { -1 } else if x > 0.4 { 1 } else { 0 };
                if stick != 0 && stick != self.stick {
                    self.cmds.push(Cmd::PadSteer);
                }
                self.stick = stick;
                self.pad = [
                    pad.is_pressed(Button::DPadLeft) || x < -0.4,
                    pad.is_pressed(Button::DPadRight) || x > 0.4,
                    pad.is_pressed(Button::East) || pad.is_pressed(Button::RightTrigger),
                    pad.is_pressed(Button::South) || pad.is_pressed(Button::West),
                ];
            }
        }
    }

    /// This frame's controls. Clears the presses.
    pub fn controls(&mut self, now: Instant) -> crate::game::Controls {
        let mut held = [false; KEYS];
        for (d, h) in held.iter_mut().enumerate() {
            let k = &mut self.keys[d];
            let down = if self.exact { k.down } else { held_by_repeat(k, now) };
            *h = down || k.latched || self.pad[d];
            k.latched = false;
        }
        let button = self.button || self.button_latched;
        self.button_latched = false;
        crate::game::Controls { left: held[LEFT], right: held[RIGHT], energy: held[ENERGY], band: held[BAND], mouse_x: self.mouse_x, button }
    }
}

/// A gamepad has the buttons Glider asks for. gilrs lists other joystick
/// devices too: a 3Dconnexion SpaceMouse has axes and a button but no A and
/// no Start, and counted as a pad, nudging it steered the glider.
#[cfg(feature = "gamepad")]
fn is_pad(p: &gilrs::Gamepad) -> bool {
    p.button_code(gilrs::Button::South).is_some() && p.button_code(gilrs::Button::Start).is_some()
}

#[cfg(feature = "gamepad")]
fn first_pad(g: &gilrs::Gilrs) -> Option<String> {
    g.gamepads().find(|(_, p)| is_pad(p)).map(|(_, p)| p.name().to_string())
}

fn held_by_repeat(k: &Key, now: Instant) -> bool {
    let Some(last) = k.last else { return false };
    let window = if k.repeats == 0 { PRE_REPEAT } else { REPEAT_GAP };
    now.duration_since(last) <= window
}
