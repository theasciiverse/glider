mod assets;
mod audio;
mod draw;
mod fx;
mod game;
mod gfx;
mod house;
mod input;
mod mac;
mod save;
#[cfg(test)]
mod tests;
mod view;

use assets::Assets;
use audio::Audio;
use fx::Show;
use game::{Controls, Fx, Game, Method};
use gfx::Canvas;
use house::{House, Library};
use input::{Cmd, Input};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event, KeyboardEnhancementFlags, MouseButton,
    MouseEventKind, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use save::{Score, Scores, Settings};
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The Mac's tick.
const TICK: Duration = Duration::from_micros(16_667);
const SNAPSHOT_EVERY: Duration = Duration::from_millis(500);

const HELP: &str = "glider: Glider 4.0 (John Calhoun, 1991) in the terminal

Fly a paper glider through the rooms of a house. Floor vents lift you,
everything else is in the way.

usage: glider [options] [HOUSE]

  HOUSE           a built-in house by name (\"Mad House\") or a Glider 4
                  house file on disk; its next files are looked for beside it
  --houses        list the built-in houses
  --mute          no sound
  --watch         no controls: for houses that play themselves
                  (Hands-off House)
  --version       print the version

Keys: left and right (arrows, or , and .) fly; Up or Enter burns battery;
Space or Down fires a rubber band. Or steer with the mouse: left or right
of the room's middle, the button boosts. Tab pauses, M sound, B music,
Q ends the game.

Logging: RUST_LOG=debug glider 2>glider.log";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Title,
    Houses,
    Help(usize),
    Playing,
    Name,
    Scores,
}

struct App {
    a: Assets,
    audio: Audio,
    input: Input,
    settings: Settings,
    scores: Scores,
    screen: Screen,
    lib: Library,
    house_name: Vec<u8>,
    house: House,
    game: Option<Game>,
    room_pic: Canvas,
    shown: Canvas,
    shows: Vec<Show>,
    paused: bool,
    watch: bool,
    ozma: usize,
    sel: usize,
    houses: Vec<&'static str>,
    name: String,
    last_score: Option<Score>,
    placed: view::Placed,
    aspect: f32,
    quit: bool,
    seed: u32,
    ticks: u64,
}

fn seed_now() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as u32 ^ d.subsec_nanos()).unwrap_or(1)
}

impl App {
    fn method(&self) -> Method {
        if self.input.exact { Method::HoldKey } else { Method::DropKey }
    }

    fn house_label(&self) -> String {
        mac::mac_roman(&self.house_name)
    }

    fn start_game(&mut self) {
        let mut g = Game::new(self.lib.clone(), &self.house_name, self.house.clone(), self.method(), seed_now());
        g.start(self.ozma >= 4);
        log::info!("game start house={:?} method={:?} ozma={}", self.house_label(), g.method, self.ozma >= 4);
        self.ozma = 0;
        self.input.clear();
        self.paused = false;
        self.screen = Screen::Playing;
        self.game = Some(g);
        self.after_step();
    }

    /// Sounds, a new room picture and transitions after the game moved.
    fn after_step(&mut self) {
        let Some(g) = &mut self.game else { return };
        for s in g.sounds.drain(..) {
            self.audio.play(s);
        }
        if g.room_dirty {
            self.room_pic = draw::room_picture(&self.a, g);
            g.room_dirty = false;
        }
        let fxs: Vec<Fx> = g.fx.drain(..).collect();
        if fxs.is_empty() {
            return;
        }
        let mut still = self.room_pic.clone();
        draw::headline(&mut still, &self.a, g);
        let score = g.score();
        for f in fxs {
            log::info!("fx {f:?}");
            let from = self.shows.last().map_or_else(|| self.shown.clone(), |s| s.canvas.clone());
            // a show waiting behind another starts from where that one ends
            let from = if self.shows.is_empty() { from } else { self.final_of_last().unwrap_or(from) };
            self.shows.push(Show::new(f, &self.a, from, still.clone(), score, seed_now()));
        }
    }

    /// What the last queued show leaves on the screen.
    fn final_of_last(&self) -> Option<Canvas> {
        let s = self.shows.last()?;
        // scrolls and dissolves end on their new picture; the rest on their own
        Some(match s.kind {
            Fx::TimeBonus(_) => s.canvas.clone(),
            _ => {
                let mut c = self.room_pic.clone();
                if let Some(g) = &self.game {
                    draw::headline(&mut c, &self.a, g);
                }
                c
            }
        })
    }

    fn game_over(&mut self) {
        let Some(g) = self.game.take() else { return };
        let house = mac::mac_roman(&g.first_house);
        let rooms = g.rooms_passed.max(0);
        let room = mac::mac_roman(&g.room().name);
        log::info!("game over house={house:?} score={} rooms={rooms}", g.score());
        let s = Score { house: house.clone(), score: g.score(), rooms, name: self.settings.name.clone(), room };
        if !self.watch && self.scores.qualifies(&house, s.score) {
            self.name = self.settings.name.clone();
            self.last_score = Some(s);
            self.screen = Screen::Name;
        } else {
            self.last_score = None;
            self.screen = Screen::Scores;
        }
        self.input.clear();
    }

    /// One tick of real time.
    fn tick(&mut self, now: Instant) {
        self.ticks += 1;
        if let Some(show) = self.shows.first_mut() {
            let mut sounds = Vec::new();
            let done = show.tick(&self.a, &mut sounds);
            for s in sounds {
                self.audio.play(s);
            }
            self.shown = show.canvas.clone();
            if done {
                let s = self.shows.remove(0);
                if s.kind == Fx::GameOver {
                    self.shows.clear();
                    self.game_over();
                }
            }
            return;
        }
        if self.screen != Screen::Playing || self.paused || self.ticks % 2 != 0 {
            return;
        }
        let c = if self.watch { Controls::default() } else { self.input.controls(now) };
        if let Some(g) = &mut self.game {
            g.step(c);
        }
        self.after_step();
        if let Some(g) = &self.game {
            if self.shows.is_empty() {
                self.shown = draw::scene(&self.a, g, &self.room_pic);
            }
            if !g.playing && self.shows.is_empty() {
                self.game_over();
            }
        }
    }

    fn snapshot(&self) {
        if let Some(g) = &self.game {
            let gl = &g.glider;
            log::debug!(
                "state screen={:?} paused={} room={} passed={} score={} mortals={} mode={} phase={} dest=({},{},{},{}) vel={} right={} bands={} energy={} ticks={} shows={}",
                self.screen,
                self.paused,
                g.room_at,
                g.rooms_passed,
                g.score(),
                g.mortals,
                gl.mode,
                gl.phase,
                gl.dest.left,
                gl.dest.top,
                gl.dest.right,
                gl.dest.bottom,
                gl.for_vel,
                gl.is_right,
                gl.bands,
                gl.energy,
                g.ticks,
                self.shows.len()
            );
        } else {
            log::debug!("state screen={:?}", self.screen);
        }
    }

    fn handle(&mut self, now: Instant) {
        let cmds: Vec<Cmd> = self.input.cmds.drain(..).collect();
        for c in cmds {
            if c == Cmd::Exit {
                self.quit = true;
                return;
            }
            match c {
                Cmd::Sound => {
                    self.audio.sound = !self.audio.sound;
                    self.settings.sound = self.audio.sound;
                    continue;
                }
                Cmd::Music => {
                    self.audio.music = !self.audio.music;
                    self.settings.music = self.audio.music;
                    continue;
                }
                _ => {}
            }
            if let Some(s) = self.shows.first() {
                // the closing and game over listen for a key; the rest run on
                if s.skippable() && matches!(c, Cmd::Any | Cmd::Confirm) {
                    let s = self.shows.remove(0);
                    if s.kind == Fx::GameOver {
                        self.shows.clear();
                        self.game_over();
                    } else if let Some(n) = self.shows.first_mut() {
                        n.canvas = s.canvas;
                    }
                }
                continue;
            }
            match self.screen {
                Screen::Title => match c {
                    Cmd::Confirm => self.start_game(),
                    Cmd::Quit | Cmd::Back => self.quit = true,
                    Cmd::Houses => {
                        self.sel = self.houses.iter().position(|h| h.as_bytes() == self.house_name).unwrap_or(0);
                        self.screen = Screen::Houses;
                    }
                    Cmd::Help => self.screen = Screen::Help(1),
                    Cmd::Scores => self.screen = Screen::Scores,
                    Cmd::Char(ch) => {
                        // "Ozma" on the title screen: fifty gliders, as in Glider
                        let code = ['O', 'z', 'm', 'a'];
                        self.ozma = if ch == code[self.ozma.min(3)] {
                            self.ozma + 1
                        } else if ch == 'O' {
                            1
                        } else {
                            0
                        };
                        if self.ozma == 4 {
                            log::info!("ozma");
                        }
                    }
                    _ => {}
                },
                Screen::Houses => match c {
                    Cmd::Up => self.sel = self.sel.saturating_sub(1),
                    Cmd::Down => self.sel = (self.sel + 1).min(self.houses.len().saturating_sub(1)),
                    Cmd::Confirm => {
                        let n = self.houses[self.sel];
                        if let Some(h) = Library::default().load(n.as_bytes()) {
                            self.lib = Library::default();
                            self.house = h;
                            self.house_name = n.as_bytes().to_vec();
                            self.settings.house = n.to_string();
                        }
                        self.screen = Screen::Title;
                    }
                    Cmd::Back | Cmd::Quit => self.screen = Screen::Title,
                    _ => {}
                },
                Screen::Help(p) => match c {
                    Cmd::Confirm | Cmd::Char(' ') | Cmd::Down => self.screen = if p >= 7 { Screen::Title } else { Screen::Help(p + 1) },
                    Cmd::Up => self.screen = Screen::Help(p.saturating_sub(1).max(1)),
                    Cmd::Back | Cmd::Quit | Cmd::Help => self.screen = Screen::Title,
                    _ => {}
                },
                Screen::Playing => {
                    if self.paused {
                        match c {
                            Cmd::Pause | Cmd::Back | Cmd::Confirm => {
                                self.paused = false;
                                self.input.clear();
                            }
                            Cmd::Quit => self.end_game(),
                            _ => {}
                        }
                    } else {
                        match c {
                            Cmd::Pause | Cmd::Back => {
                                self.paused = true;
                                log::info!("pause");
                            }
                            Cmd::Quit => self.end_game(),
                            Cmd::Steer => self.set_method(self.method()),
                            _ => {}
                        }
                    }
                }
                Screen::Name => match c {
                    Cmd::Confirm => {
                        if let Some(mut s) = self.last_score.take() {
                            s.name = if self.name.trim().is_empty() { "Anonymous".into() } else { self.name.trim().to_string() };
                            self.settings.name = s.name.clone();
                            self.scores.add(s.clone());
                            self.last_score = Some(s);
                        }
                        self.screen = Screen::Scores;
                    }
                    Cmd::Backspace => {
                        self.name.pop();
                    }
                    Cmd::Char(ch) if !ch.is_control() && self.name.chars().count() < save::NAME_MAX => self.name.push(ch),
                    Cmd::Back => {
                        self.last_score = None;
                        self.screen = Screen::Scores;
                    }
                    _ => {}
                },
                Screen::Scores => {
                    if matches!(c, Cmd::Any | Cmd::Confirm | Cmd::Back) {
                        self.screen = Screen::Title;
                        self.shown = self.title_picture();
                    }
                }
            }
        }
        if self.screen == Screen::Playing && !self.paused && self.input.mouse_moved && !self.watch {
            self.input.mouse_moved = false;
            self.set_method(Method::AbsMouse);
        }
        let _ = now;
    }

    fn set_method(&mut self, m: Method) {
        if let Some(g) = &mut self.game
            && g.method != m
        {
            log::info!("control method {m:?}");
            g.method = m;
        }
    }

    /// End Game: you may still get on the high scores.
    fn end_game(&mut self) {
        self.paused = false;
        if let Some(g) = &mut self.game {
            g.playing = false;
            let score = g.score();
            self.shows.push(Show::new(Fx::GameOver, &self.a, self.shown.clone(), Canvas::new(), score, seed_now()));
        }
    }

    fn title_picture(&self) -> Canvas {
        let mut c = Canvas::new();
        c.copy_bits(&self.a.idle, house::Rect::new(0, 0, 512, 342), gfx::WHOLE);
        c
    }

    fn on_mouse(&mut self, kind: MouseEventKind, col: u16, row: u16) {
        let x = if self.placed.contains(col, row) { self.placed.room_x(col) } else { None };
        match kind {
            MouseEventKind::Moved | MouseEventKind::Drag(_) => self.input.mouse_move(x),
            MouseEventKind::Down(MouseButton::Left) => {
                self.input.mouse_move(x);
                self.input.mouse_down();
            }
            MouseEventKind::Up(MouseButton::Left) => self.input.mouse_up(),
            _ => {}
        }
    }

    fn draw(&mut self, buf: &mut Buffer, area: Rect) {
        let bg = Style::default().bg(Color::Rgb(0, 0, 0)).fg(Color::Rgb(200, 200, 200));
        buf.set_style(area, bg);
        let text_rows = if area.height > 12 { 1 } else { 0 };
        let pic_area = Rect::new(area.x, area.y, area.width, area.height - text_rows);
        let placed = view::place(pic_area, self.aspect);
        self.placed = placed;
        let dim = Style::default().fg(Color::Rgb(150, 150, 150)).bg(Color::Rgb(0, 0, 0));
        let hi = Style::default().fg(Color::Rgb(255, 210, 90)).bg(Color::Rgb(0, 0, 0)).add_modifier(Modifier::BOLD);
        let status_y = placed.cells.bottom().min(area.bottom().saturating_sub(1));
        match self.screen {
            Screen::Title | Screen::Playing if !self.shows.is_empty() || self.screen == Screen::Playing => {
                view::draw(buf, &self.shown, &placed);
                if text_rows > 0 {
                    let line = if self.paused {
                        "Paused.  Tab or Enter to fly on, Q to end the game".to_string()
                    } else if let Some(g) = &self.game {
                        format!(
                            "{}  room {} of {}   Tab pause  M sound {}  B music {}",
                            self.house_label(),
                            g.room_at.max(0),
                            g.house.n_rooms,
                            on_off(self.audio.sound),
                            on_off(self.audio.music)
                        )
                    } else {
                        String::new()
                    };
                    put_centred(buf, area, status_y, &line, if self.paused { hi } else { dim });
                }
                if self.paused {
                    let y = placed.cells.y + placed.cells.height / 2;
                    put_centred(buf, area, y, "  P A U S E D  ", hi);
                }
            }
            Screen::Title | Screen::Playing => {
                view::draw(buf, &self.title_picture(), &placed);
                if text_rows > 0 {
                    let s = format!(
                        "{}   Enter fly  L house  T scores  ? help  M sound {}  B music {}  Q quit",
                        self.house_label(),
                        on_off(self.audio.sound),
                        on_off(self.audio.music)
                    );
                    put_centred(buf, area, status_y, &s, hi);
                }
            }
            Screen::Houses => {
                put_centred(buf, area, area.y + 1, "Choose a house", hi);
                for (i, h) in self.houses.iter().enumerate() {
                    let y = area.y + 3 + i as u16;
                    if y >= area.bottom() {
                        break;
                    }
                    let rooms = Library::default().load(h.as_bytes()).map_or(0, |x| x.n_rooms);
                    let s = format!("{} {:<34} {:>2} rooms", if i == self.sel { ">" } else { " " }, h, rooms);
                    put_centred(buf, area, y, &s, if i == self.sel { hi } else { dim });
                }
                put_centred(buf, area, area.bottom().saturating_sub(1), "Up and Down to choose, Enter to take it, Esc to go back", dim);
            }
            Screen::Help(p) => {
                let mut c = Canvas::new();
                c.fill(gfx::WHOLE, gfx::WHITE);
                let logo = &self.a.help[0];
                c.copy_bits(logo, house::Rect::new(0, 0, logo.w, logo.h), house::Rect::new(177, 10, 177 + logo.w, 10 + logo.h));
                let page = &self.a.help[p.min(7)];
                c.copy_bits(page, house::Rect::new(0, 0, page.w, page.h), house::Rect::new(4, 100, 4 + page.w, 100 + page.h));
                view::draw(buf, &c, &placed);
                put_centred(buf, area, status_y, &format!("Help page {p} of 7.  Space next, Up back, Esc done"), dim);
            }
            Screen::Name => {
                let score = self.last_score.as_ref().map_or(0, |s| s.score);
                let y = area.y + area.height / 2;
                put_centred(buf, area, y.saturating_sub(2), &format!("A high score for {}: {score}", self.house_label()), hi);
                put_centred(buf, area, y, &format!("Your name: {}_", self.name), hi);
                put_centred(buf, area, y + 2, "Enter to keep it", dim);
            }
            Screen::Scores => {
                let house = self.game.as_ref().map_or_else(|| self.house_label(), |g| mac::mac_roman(&g.first_house));
                put_centred(buf, area, area.y + 1, &format!("High scores: {house}"), hi);
                let t = self.scores.table(&house);
                if t.is_empty() {
                    put_centred(buf, area, area.y + 3, "No scores yet.", dim);
                }
                for (i, s) in t.iter().enumerate() {
                    let y = area.y + 3 + i as u16;
                    if y >= area.bottom().saturating_sub(1) {
                        break;
                    }
                    let mine = self.last_score.as_ref().is_some_and(|l| l == s);
                    let line = format!("{:>2}. {:<24} {:>8}  {:>3} rooms  {}", i + 1, s.name, s.score, s.rooms, s.room);
                    put_centred(buf, area, y, &line, if mine { hi } else { dim });
                }
                put_centred(buf, area, area.bottom().saturating_sub(1), "Any key", dim);
            }
        }
    }
}

fn on_off(b: bool) -> &'static str {
    if b { "on" } else { "off" }
}

fn put(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style) {
    let area = buf.area;
    if y < area.y || y >= area.bottom() {
        return;
    }
    let mut cx = x;
    for ch in s.chars() {
        if cx >= area.right() {
            break;
        }
        if cx >= area.x {
            buf[(cx, y)].set_char(ch).set_style(style);
        }
        cx += 1;
    }
}

fn put_centred(buf: &mut Buffer, r: Rect, y: u16, s: &str, style: Style) {
    let w = s.chars().count() as u16;
    put(buf, r.x + r.width.saturating_sub(w) / 2, y, s, style);
}

fn main() -> io::Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).filter(|v| !v.starts_with("--")).cloned();
    if has("--help") || has("-h") {
        println!("{HELP}");
        return Ok(());
    }
    if has("--version") {
        println!("glider {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if has("--houses") {
        for h in house::starting_houses() {
            println!("{h}");
        }
        return Ok(());
    }
    let settings = Settings::load();
    // a house named on the command line: built in, or a file
    let wanted = args.iter().rev().find(|a| !a.starts_with("--") && Some(*a) != value("--shot").as_ref()).cloned();
    let (lib, house_name, house) = match &wanted {
        Some(w) if PathBuf::from(w).is_file() => match Library::open(w.as_ref()) {
            Some(x) => x,
            None => {
                eprintln!("glider: {w} is not a Glider 4 house");
                std::process::exit(1);
            }
        },
        Some(w) => match Library::default().load(w.as_bytes()) {
            Some(h) => (Library::default(), w.as_bytes().to_vec(), h),
            None => {
                eprintln!("glider: no house called {w:?} (glider --houses lists them)");
                std::process::exit(1);
            }
        },
        None => {
            let name = settings.house.clone();
            match Library::default().load(name.as_bytes()) {
                Some(h) => (Library::default(), name.into_bytes(), h),
                None => (Library::default(), b"The House".to_vec(), Library::default().load(b"The House").expect("The House is built in")),
            }
        }
    };

    let a = Assets::load();
    // open audio before the alternate screen, so any ALSA chatter lands above it
    let audio = Audio::new(has("--mute"), a.sounds.clone(), settings.sound, settings.music);
    let mut app = App {
        audio,
        input: Input::new(false),
        scores: Scores::load(),
        screen: Screen::Title,
        lib,
        house_name,
        house,
        game: None,
        room_pic: Canvas::new(),
        shown: Canvas::new(),
        shows: Vec::new(),
        paused: false,
        watch: has("--watch"),
        ozma: 0,
        sel: 0,
        houses: house::starting_houses(),
        name: String::new(),
        last_score: None,
        placed: view::Placed::default(),
        aspect: view::cell_aspect(),
        quit: false,
        seed: seed_now(),
        ticks: 0,
        settings,
        a,
    };
    app.shown = app.title_picture();
    let _ = app.seed;

    if let Some(n) = value("--shot").and_then(|v| v.parse::<u32>().ok()) {
        shot(&mut app, n, value("--size"));
        return Ok(());
    }

    let mut terminal = ratatui::init();
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_EVENT_TYPES | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    let _ = execute!(io::stdout(), EnableFocusChange, EnableMouseCapture);
    app.input.exact = kitty;
    app.input.open_pad();
    log::info!("start kitty_keyboard={kitty} pad={:?} audio={} house={:?}", app.input.pad_name, app.audio.available(), app.house_label());
    if app.watch {
        app.start_game();
    }
    let result = run(&mut terminal, &mut app);

    let _ = execute!(io::stdout(), DisableMouseCapture, DisableFocusChange);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    app.settings.save();
    result
}

/// `--shot STEPS [--size WxH]`, a developer's tool: the chosen house played
/// with no controls for that many frames (transitions skipped), drawn into
/// a WxH screen of cells twice as tall as wide and printed with colour
/// escapes, no terminal needed.
fn shot(app: &mut App, steps: u32, size: Option<String>) {
    let (w, h) = size.and_then(|v| v.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))).unwrap_or((128u16, 44u16));
    app.aspect = 2.0;
    app.start_game();
    for _ in 0..steps {
        if let Some(g) = &mut app.game {
            if !g.playing {
                break;
            }
            g.step(Controls::default());
            g.sounds.clear();
            g.fx.clear();
            if g.room_dirty {
                app.room_pic = draw::room_picture(&app.a, g);
                g.room_dirty = false;
            }
        }
    }
    app.shows.clear();
    if let Some(g) = &app.game {
        app.shown = draw::scene(&app.a, g, &app.room_pic);
    }
    let area = Rect::new(0, 0, w, h);
    let mut buf = Buffer::empty(area);
    app.draw(&mut buf, area);
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            let c = &buf[(x, y)];
            for (code, col) in [(38, c.fg), (48, c.bg)] {
                if let Color::Rgb(r, g, b) = col {
                    out.push_str(&format!("\x1b[{code};2;{r};{g};{b}m"));
                }
            }
            out.push_str(c.symbol());
        }
        out.push_str("\x1b[0m\n");
    }
    print!("{out}");
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut next = Instant::now();
    let mut snapshot = Instant::now();
    while !app.quit {
        loop {
            let wait = next.saturating_duration_since(Instant::now());
            if !event::poll(wait)? {
                break;
            }
            match event::read()? {
                Event::Key(k) => {
                    log::debug!("key {:?} {:?} {:?}", k.code, k.kind, k.modifiers);
                    app.input.on_key(k, Instant::now());
                }
                Event::Mouse(m) => {
                    if !matches!(m.kind, MouseEventKind::Moved) {
                        log::debug!("mouse {:?} at {},{}", m.kind, m.column, m.row);
                    }
                    app.on_mouse(m.kind, m.column, m.row);
                }
                Event::FocusLost => {
                    log::debug!("focus lost");
                    app.input.release_all();
                    if app.screen == Screen::Playing && !app.watch {
                        app.paused = true;
                    }
                }
                Event::Resize(w, h) => {
                    app.aspect = view::cell_aspect();
                    log::info!("resize {w}x{h} aspect {:.3}", app.aspect);
                }
                _ => {}
            }
        }
        app.input.poll_pad();
        let now = Instant::now();
        app.handle(now);
        // catch up four ticks at most if the terminal was slow
        let mut ticks = 0;
        while now >= next && ticks < 4 {
            app.tick(now);
            next += TICK;
            ticks += 1;
        }
        if now >= next {
            next = now + TICK;
        }
        if now.duration_since(snapshot) >= SNAPSHOT_EVERY {
            snapshot = now;
            app.snapshot();
        }
        if app.ticks % 2 == 0 || ticks > 1 {
            terminal.draw(|f| {
                let area = f.area();
                app.draw(f.buffer_mut(), area);
            })?;
        }
    }
    Ok(())
}
