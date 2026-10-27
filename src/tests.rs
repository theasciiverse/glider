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
