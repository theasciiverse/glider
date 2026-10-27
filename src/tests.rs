use crate::assets::Assets;
use crate::draw;
use crate::game::{Controls, Fx, Game, Method};
use crate::house::{BUILT_IN, House, Library};

/// Play a house with no controls until the game ends or `max` frames pass.
/// Returns the game, every transition it asked for, and the rooms where
/// gliders were lost.
fn watch(name: &str, seed: u32, max: usize) -> (Game, Vec<Fx>, Vec<i32>) {
    let lib = Library::default();
    let h = lib.load(name.as_bytes()).unwrap();
    let mut g = Game::new(lib, name.as_bytes(), h, Method::HoldKey, seed);
    g.start(false);
    let mut seen = Vec::new();
    let mut lost = Vec::new();
    for _ in 0..max {
        if !g.playing {
            break;
        }
        let (m, room) = (g.mortals, g.room_at);
        g.step(Controls::default());
        g.sounds.clear();
        seen.append(&mut g.fx);
        if g.mortals < m {
            lost.push(room);
        }
    }
    (g, seen, lost)
}

/// Hands-off House (Ward Hartenstein): "a 40 room Glider house that
/// actually PLAYS ITSELF ... AND IT NEVER CRASHES!" Untouched, the glider
/// must fly every room to the end without losing one (it picks up extra
/// gliders on the way). Its open window rooms are random, so several seeds.
/// It was tuned on a slower Mac; with the patches in `house.rs` it plays
/// itself here too.
#[test]
fn hands_off_house_plays_itself() {
    for seed in [1, 12345, 987_654_321] {
        let (g, fx, lost) = watch("Hands-off House", seed, 60 * 60 * 30);
        assert!(!g.playing, "seed {seed}: still flying in room {} after an hour", g.room_at);
        assert!(fx.contains(&Fx::Closing), "seed {seed}: the house did not end with the closing animation (room {}, {} gliders)", g.room_at, g.mortals);
        assert!(lost.is_empty(), "seed {seed}: gliders lost in rooms {lost:?}");
    }
}

#[test]
fn every_room_of_every_house_sets_up_and_draws() {
    let a = Assets::load();
    for (name, d) in BUILT_IN {
        let h = House::parse(d).unwrap();
        let mut g = Game::new(Library::default(), name.as_bytes(), h.clone(), Method::HoldKey, 7);
        g.start(false);
        for r in 1..=h.n_rooms {
            g.room_at = r;
            g.house = h.clone();
            // ReadyRoom by way of a transport into the room
            g.glider.dest = crate::house::Rect::new(232, 24, 280, 44);
            for _ in 0..3 {
                g.step(Controls::default());
            }
            for look in [draw::Look::Original, draw::Look::Silhouette] {
                let pic = draw::room_picture(&a, &g, look);
                let _ = draw::scene(&a, &g, &pic, look);
            }
        }
    }
}

#[test]
fn a_glider_left_alone_sinks_and_crashes() {
    // The House, room 1: no controls, the glider fades in and drifts down
    // into the floor (or the vents hold it), but never leaves the room
    let (g, _, _) = watch("The House", 3, 30 * 20);
    assert_eq!(g.room_at, 1);
}

/// A developer's tool: every frame of the first pass through a room of
/// Hands-off House (ROOM, default 13), until the glider leaves or dies.
/// `ROOM=7 cargo test --release trace_hands_off_room -- --ignored --nocapture`
#[test]
#[ignore]
fn trace_hands_off_room() {
    let room: i32 = std::env::var("ROOM").ok().and_then(|v| v.parse().ok()).unwrap_or(13);
    let lib = Library::default();
    let h = lib.load(b"Hands-off House").unwrap();
    let mut g = Game::new(lib, b"Hands-off House", h, Method::HoldKey, 1);
    g.frame_ticks = std::env::var("FT").ok().and_then(|v| v.parse().ok()).unwrap_or(2.0);
    g.start(false);
    let mut entered = None;
    for f in 0..200_000 {
        if !g.playing {
            break;
        }
        let before = g.mortals;
        g.step(Controls::default());
        g.sounds.clear();
        let fx: Vec<Fx> = g.fx.drain(..).collect();
        if g.room_at == room && entered.is_none() {
            entered = Some(g.ticks);
        }
        let Some(t0) = entered else { continue };
        let gl = &g.glider;
        let outlets: Vec<String> =
            g.cycles.iter().filter(|c| c.kind == crate::game::OUTLET).map(|c| format!("{}:{}", c.tied_to, if c.accel == 25 { "-" } else { "Z" })).collect();
        println!(
            "f{f} t+{} room {} mode {} ph {} dest ({},{},{},{}) vel {} life {} outlets {:?} fx {:?}",
            g.ticks - t0,
            g.room_at,
            gl.mode,
            gl.phase,
            gl.dest.left,
            gl.dest.top,
            gl.dest.right,
            gl.dest.bottom,
            gl.for_vel,
            g.life_normal,
            outlets,
            fx
        );
        if g.mortals < before || g.room_at != room {
            break;
        }
    }
}

/// A developer's tool: where Hands-off House first loses a glider, by
/// ticks per frame (FT, several as "2.0,2.5"; default 2.0 to 4.0 in tenths)
/// and the ReadyRoom estimate (RR, default 0), over five seeds.
#[test]
#[ignore]
fn sweep_ready_room_ticks() {
    let list = |k: &str| std::env::var(k).ok().map(|v| v.split(',').filter_map(|x| x.parse::<f64>().ok()).collect::<Vec<_>>());
    let fts = list("FT").unwrap_or_else(|| (20..=40).map(|t| t as f64 / 10.0).collect());
    let rrs = list("RR").unwrap_or_else(|| vec![0.0]);
    for &ft in &fts {
        for &rr in &rrs {
            let mut results = Vec::new();
            for seed in [1u32, 12345, 987_654_321, 42, 7] {
                let lib = Library::default();
                let h = lib.load(b"Hands-off House").unwrap();
                let mut g = Game::new(lib, b"Hands-off House", h, Method::HoldKey, seed);
                g.ready_room_ticks = rr as i64;
                g.frame_ticks = ft;
                g.start(false);
                let mut first_death = None;
                for _ in 0..200_000 {
                    if !g.playing {
                        break;
                    }
                    let m = g.mortals;
                    g.step(Controls::default());
                    g.sounds.clear();
                    g.fx.clear();
                    if g.mortals < m && first_death.is_none() {
                        first_death = Some(g.room_at);
                    }
                }
                results.push(match first_death {
                    None if !g.playing => "WON".to_string(),
                    None => "stuck".to_string(),
                    Some(r) => format!("r{r}"),
                });
            }
            println!("frame_ticks {ft:.1} ready_room {rr:2}: {}", results.join(" "));
        }
    }
}

#[test]
fn in_game_text_is_terminal_text() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    let a = Assets::load();
    let show = crate::fx::Show::new(Fx::TimeBonus(192), &a, crate::gfx::Canvas::new(), crate::gfx::Canvas::new(), 0, 1);
    let area = Rect::new(0, 0, 128, 44);
    let p = crate::view::place(area, 2.0);
    let mut buf = Buffer::empty(area);
    crate::view::draw_labels(&mut buf, &p, &show.label().into_iter().collect::<Vec<_>>());
    let text: String = (0..area.height).map(|y| (0..area.width).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>() + "\n").collect();
    assert!(text.contains("Time Bonus = 192"), "{text}");
}

/// In the silhouette view a 'copter's two-pixel blades stay on screen,
/// the area drawn near the sprite's own, and the glider's lower wing is
/// its shade, not its ink.
#[test]
fn silhouettes_keep_thin_parts_and_shades() {
    use crate::gfx::{Canvas, shade};
    use crate::house::Rect as R;
    use ratatui::buffer::Buffer;
    use ratatui::style::Color;
    let a = Assets::load();
    let mut c = Canvas::new();
    c.fill(crate::gfx::WHOLE, [150, 140, 120]);
    c.fore = vec![None; c.px.len()];
    let (copter, glider) = ([205, 205, 225], [250, 250, 255]);
    c.ink = Some(copter);
    c.sprite(&a, R::new(160, 0, 192, 32), R::new(194, 60, 226, 92));
    c.ink = Some(glider);
    c.sprite(&a, R::new(0, 0, 48, 20), R::new(300, 100, 348, 120));
    c.ink = None;
    let p = crate::view::place(ratatui::layout::Rect::new(0, 0, 128, 44), 2.0);
    let mut buf = Buffer::empty(p.cells);
    crate::view::draw_braille(&mut buf, &c, &p);
    let (w, h) = (p.cells.width as i32, p.cells.height as i32);
    let shows = |col: [u8; 3], x: i32, y: i32| {
        let col = Color::Rgb(col[0], col[1], col[2]);
        let cell = &buf[(p.cells.x + x as u16, p.cells.y + y as u16)];
        (cell.symbol() != " " && cell.fg == col) || cell.bg == col
    };
    // the 'copter's area as drawn, in a cell's 8x16 points, against the
    // sprite's own
    let mut drawn = 0;
    for y in 0..h {
        for x in 0..w {
            let cell = &buf[(p.cells.x + x as u16, p.cells.y + y as u16)];
            let ch = cell.symbol().chars().next().unwrap_or(' ');
            let n = crate::view::points(ch);
            if cell.bg == Color::Rgb(copter[0], copter[1], copter[2]) {
                drawn += 128 - n;
            } else if cell.fg == Color::Rgb(copter[0], copter[1], copter[2]) {
                drawn += n;
            }
        }
    }
    let pixels = (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).filter(|&(x, y)| a.mask_at(160 + x, y)).count() as f32;
    let area = pixels * 128.0 * (w * h) as f32 / (512.0 * 342.0);
    assert!(drawn as f32 > area * 0.8, "the 'copter drawn {drawn} points, its sprite covers {area}");
    let glider_has = |col| (100 * h / 342..120 * h / 342 + 1).any(|y| (300 * w / 512..348 * w / 512 + 1).any(|x| shows(col, x, y)));
    assert!(glider_has(shade(glider)), "no shaded wing");
    assert!(glider_has(glider), "no lit wing");
}

/// Room 1 of The House drawn in the silhouette view at 128x44, and the
/// cells over the first object of `kind` in it.
fn cells_over(kind: i32) -> Vec<ratatui::buffer::Cell> {
    let lib = Library::default();
    let h = lib.load(b"The House").unwrap();
    let mut g = Game::new(lib, b"The House", h, Method::HoldKey, 1);
    g.start(false);
    let a = Assets::load();
    let room = g.room().clone();
    let b = room.objects[..room.n_objects as usize].iter().find(|o| o.kind == kind).expect("the object is in room 1").bound;
    let c = draw::scene(&a, &g, &draw::room_picture(&a, &g, draw::Look::Silhouette), draw::Look::Silhouette);
    let p = crate::view::place(ratatui::layout::Rect::new(0, 0, 128, 44), 2.0);
    let mut buf = ratatui::buffer::Buffer::empty(p.cells);
    crate::view::draw_braille(&mut buf, &c, &p);
    let (w, h) = (p.cells.width as i32, p.cells.height as i32);
    (b.top * h / 342..b.bottom * h / 342)
        .flat_map(|y| (b.left * w / 512..b.right * w / 512).map(move |x| (x, y)))
        .map(|(x, y)| buf[(p.cells.x + x as u16, p.cells.y + y as u16)].clone())
        .collect()
}

fn braille(c: &ratatui::buffer::Cell) -> bool {
    c.symbol().chars().any(|ch| ('\u{2800}'..='\u{28FF}').contains(&ch))
}

fn shows(cells: &[ratatui::buffer::Cell], col: [u8; 3]) -> bool {
    let col = ratatui::style::Color::Rgb(col[0], col[1], col[2]);
    cells.iter().any(|c| c.bg == col || (c.fg == col && c.symbol() != " "))
}

/// The clock in The House's first room, in the silhouette view: drawn in
/// solid characters (no Braille) showing its yellow rim, white face and
/// black hands.
#[test]
fn the_clock_reads_as_a_clock() {
    let cells = cells_over(crate::game::CLOCK);
    assert!(!cells.iter().any(braille), "Braille in the clock");
    for (name, col) in [("rim", [252, 243, 5]), ("face", [255, 255, 255]), ("hands", [0, 0, 0])] {
        assert!(shows(&cells, col), "no {name}");
    }
}

/// The Mac beside it is drawn by the same matcher in its art's colours,
/// its blue screen and gray case; in Braille it was a yellow smear.
#[test]
fn the_mac_is_drawn_in_its_colours() {
    let cells = cells_over(crate::game::MAC_TSH);
    assert!(!cells.iter().any(braille), "Braille in the Mac");
    assert!(shows(&cells, [2, 171, 234]), "no screen");
    assert!(shows(&cells, [192, 192, 192]), "no case");
}

/// GameOver in the silhouette view: the letters crisp and purple, not
/// Braille, and once the word has fallen, no crumbs of its earlier
/// rows left in the pile of pages above it.
#[test]
fn game_over_letters_are_sharp() {
    let lib = Library::default();
    let h = lib.load(b"The House").unwrap();
    let mut g = Game::new(lib, b"The House", h, Method::HoldKey, 1);
    g.start(false);
    let a = Assets::load();
    let c = draw::scene(&a, &g, &draw::room_picture(&a, &g, draw::Look::Silhouette), draw::Look::Silhouette);
    let mut show = crate::fx::Show::new(Fx::GameOver, &a, c, crate::gfx::Canvas::new(), 0, 7);
    while !show.tick(&a, &mut Vec::new()) {}
    let p = crate::view::place(ratatui::layout::Rect::new(0, 0, 128, 44), 2.0);
    let mut buf = ratatui::buffer::Buffer::empty(p.cells);
    crate::view::draw_braille(&mut buf, &show.canvas, &p);
    let cells: Vec<_> = buf.content().to_vec();
    let ink = ratatui::style::Color::Rgb(70, 0, 165);
    let letters: Vec<_> = cells.iter().filter(|c| c.fg == ink || c.bg == ink).collect();
    // a stroke's tip may be a single Braille dot; in Braille, every cell was
    let dotted = letters.iter().filter(|c| braille(c)).count();
    assert!(dotted * 10 < letters.len(), "{dotted} of {} letter cells in Braille", letters.len());
    let rows: Vec<u16> = (0..p.cells.height).filter(|&y| (0..p.cells.width).any(|x| buf[(p.cells.x + x, p.cells.y + y)].fg == ink || buf[(p.cells.x + x, p.cells.y + y)].bg == ink)).collect();
    assert!(!rows.is_empty(), "no letters");
    // the last row's letters are 45 pixels tall: about six cells, nothing above
    assert!(rows.last().unwrap() - rows[0] <= 7, "letters spread over rows {rows:?}");
}

/// A developer's tool: Hands-off House with a thousand gliders at each
/// speed, listing the rooms where gliders were lost and how many.
#[test]
#[ignore]
fn hands_off_deaths_by_room() {
    for ft10 in 20..=32 {
        let ft = ft10 as f64 / 10.0;
        let lib = Library::default();
        let h = lib.load(b"Hands-off House").unwrap();
        let mut g = Game::new(lib, b"Hands-off House", h, Method::HoldKey, 1);
        g.frame_ticks = ft;
        g.start(false);
        g.mortals = 1000;
        let mut deaths = std::collections::BTreeMap::new();
        let mut frames = 0;
        while g.playing && frames < 400_000 {
            let m = g.mortals;
            let room = g.room_at;
            g.step(Controls::default());
            g.sounds.clear();
            g.fx.clear();
            if g.mortals < m {
                *deaths.entry(room).or_insert(0) += 1;
                // a room that kills every time would loop forever: move on
                if deaths[&room] >= 20 {
                    break;
                }
            }
            frames += 1;
        }
        println!("ft {ft:.1}: {} rooms passed {}, deaths by room {:?}", if g.playing { "STUCK" } else { "ended" }, g.rooms_passed, deaths);
    }
}


/// Hands-off House with a change to its data, played untouched over five
/// seeds: the first room where a glider is lost, or "WON".
fn hands_off_with(patch: &dyn Fn(&mut House)) -> Vec<String> {
    [1u32, 12345, 987_654_321, 42, 7]
        .iter()
        .map(|&seed| {
            let lib = Library::default();
            let mut h = lib.load(b"Hands-off House").unwrap();
            patch(&mut h);
            let mut g = Game::new(lib, b"Hands-off House", h, Method::HoldKey, seed);
            g.start(false);
            for _ in 0..200_000 {
                if !g.playing {
                    break;
                }
                let m = g.mortals;
                g.step(Controls::default());
                g.sounds.clear();
                g.fx.clear();
                if g.mortals < m {
                    return format!("r{}", g.room_at);
                }
            }
            if g.playing { "stuck".into() } else { "WON".into() }
        })
        .collect()
}

/// A developer's tool: try values for one object's field in Hands-off House.
/// ROOM, OBJ (1-based), FIELD (amount or extra), FROM, TO, STEP.
#[test]
#[ignore]
fn search_hands_off_patch() {
    let var = |k: &str, d: i32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let (room, obj) = (var("ROOM", 7) as usize, var("OBJ", 7) as usize);
    let field = std::env::var("FIELD").unwrap_or("extra".into());
    let mut v = var("FROM", 0);
    while v <= var("TO", 120) {
        let f = field.clone();
        let base = std::env::var("BASE").unwrap_or_default();
        let r = hands_off_with(&move |h: &mut House| {
            // BASE: earlier fixes as room:obj:field:value, comma separated
            for p in base.split(',').filter(|p| !p.is_empty()) {
                let q: Vec<&str> = p.split(':').collect();
                let (r, o, val) = (q[0].parse::<usize>().unwrap(), q[1].parse::<usize>().unwrap(), q[3].parse().unwrap());
                let ob = &mut h.rooms[r].objects[o - 1];
                if q[2] == "amount" { ob.amount = val } else { ob.extra = val }
            }
            let o = &mut h.rooms[room].objects[obj - 1];
            if f == "amount" {
                o.amount = v;
            } else {
                o.extra = v;
            }
        });
        println!("room {room} obj {obj} {field} {v:4}: {}", r.join(" "));
        v += var("STEP", 5);
    }
}
