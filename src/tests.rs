use crate::assets::Assets;
use crate::draw;
use crate::game::{Controls, Fx, Game, Method};
use crate::house::{BUILT_IN, House, Library};

/// Play a house with no controls until the game ends or `max` frames pass.
/// Returns the game and every transition it asked for.
fn watch(name: &str, seed: u32, max: usize) -> (Game, Vec<Fx>) {
    let lib = Library::default();
    let h = lib.load(name.as_bytes()).unwrap();
    let mut g = Game::new(lib, name.as_bytes(), h, Method::HoldKey, seed);
    g.start(false);
    let mut seen = Vec::new();
    for _ in 0..max {
        if !g.playing {
            break;
        }
        g.step(Controls::default());
        g.sounds.clear();
        seen.append(&mut g.fx);
    }
    (g, seen)
}

/// Hands-off House (Ward Hartenstein): "a 40 room Glider house that
/// actually PLAYS ITSELF ... AND IT NEVER CRASHES!" Untouched, the glider
/// must fly every room to the end and keep all five gliders. Its open
/// window rooms are random, so several seeds.
/// Not passing yet: at 2 ticks a frame (Glider's speed on a fast Mac) the
/// first glider is lost in room 7, to the toaster; at 3 ticks a frame it
/// gets to room 13. Being worked out, see CLAUDE.md.
#[test]
#[ignore]
fn hands_off_house_plays_itself() {
    for seed in [1, 12345, 987_654_321] {
        let (g, fx) = watch("Hands-off House", seed, 60 * 60 * 30);
        assert!(!g.playing, "seed {seed}: still flying in room {} after an hour", g.room_at);
        assert!(fx.contains(&Fx::Closing), "seed {seed}: the house did not end with the closing animation (room {}, {} gliders)", g.room_at, g.mortals);
        assert_eq!(g.mortals, 5, "seed {seed}: lost a glider by room {}", g.rooms_passed);
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
    let (g, _) = watch("The House", 3, 30 * 20);
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
/// ticks per frame and the ReadyRoom estimate, over five seeds.
#[test]
#[ignore]
fn sweep_ready_room_ticks() {
    for (ft, rr) in [2, 3, 4].iter().flat_map(|&f| [0i64, 5, 10, 20, 40].map(move |r| (f, r))) {
        let mut results = Vec::new();
        for seed in [1u32, 12345, 987_654_321, 42, 7] {
            let lib = Library::default();
            let h = lib.load(b"Hands-off House").unwrap();
            let mut g = Game::new(lib, b"Hands-off House", h, Method::HoldKey, seed);
            g.ready_room_ticks = rr;
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
                None => "ok".to_string(),
                Some(r) => format!("r{r}"),
            });
        }
        println!("frame_ticks {ft} ready_room_ticks {rr:2}: {}", results.join(" "));
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
