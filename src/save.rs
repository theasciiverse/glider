//! Files kept between runs, in `$XDG_DATA_HOME/glider` (`%APPDATA%\glider`
//! on Windows, `~/Library/Application Support/glider` on macOS;
//! `GLIDER_DATA_DIR` overrides):
//!
//! - `scores.txt`, each house's high scores. Glider 4 wrote these into the
//!   house file itself (20 per house: score, rooms, name, room name); here
//!   they sit beside it, one tab-separated line each, so the houses stay
//!   as shipped.
//! - `glider.cfg`, settings as `key value` lines.

use std::path::PathBuf;

pub const NUM_SCORES: usize = 20;
pub const NAME_MAX: usize = 24;

pub fn dir() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(d) = env("GLIDER_DATA_DIR") {
        return Some(d);
    }
    let base = if cfg!(windows) {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share")))
    };
    base.map(|b| b.join("glider"))
}

fn file(name: &str) -> Option<PathBuf> {
    let d = dir()?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d.join(name))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub house: String,
    pub score: i64,
    pub rooms: i32,
    pub name: String,
    pub room: String,
}

#[derive(Default)]
pub struct Scores {
    pub all: Vec<Score>,
}

fn clean(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(NAME_MAX).collect()
}

impl Scores {
    pub fn load() -> Scores {
        let Some(text) = file("scores.txt").and_then(|p| std::fs::read_to_string(p).ok()) else { return Scores::default() };
        let all = text
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                Some(Score { house: f.first()?.to_string(), score: f.get(1)?.parse().ok()?, rooms: f.get(2)?.parse().ok()?, name: f.get(3)?.to_string(), room: f.get(4).unwrap_or(&"").to_string() })
            })
            .collect();
        Scores { all }
    }

    pub fn save(&self) {
        let Some(p) = file("scores.txt") else { return };
        let text: String = self.all.iter().map(|s| format!("{}\t{}\t{}\t{}\t{}\n", s.house, s.score, s.rooms, clean(&s.name), clean(&s.room))).collect();
        if let Err(e) = std::fs::write(&p, text) {
            log::warn!("cannot save scores to {}: {e}", p.display());
        }
    }

    /// A house's table, best first.
    pub fn table(&self, house: &str) -> Vec<Score> {
        let mut t: Vec<Score> = self.all.iter().filter(|s| s.house == house).cloned().collect();
        t.sort_by(|a, b| b.score.cmp(&a.score));
        t.truncate(NUM_SCORES);
        t
    }

    /// Would this score make the house's table?
    pub fn qualifies(&self, house: &str, score: i64) -> bool {
        let t = self.table(house);
        score > 0 && (t.len() < NUM_SCORES || t.last().is_some_and(|s| score > s.score))
    }

    pub fn add(&mut self, s: Score) {
        let house = s.house.clone();
        self.all.push(s);
        let keep = self.table(&house);
        self.all.retain(|x| x.house != house);
        self.all.extend(keep);
        self.save();
    }
}

/// Settings as `key value` lines.
#[derive(Clone, Debug)]
pub struct Settings {
    pub sound: bool,
    pub music: bool,
    pub name: String,
    pub house: String,
    /// The silhouette view rather than Calhoun's art.
    pub silhouette: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings { sound: true, music: true, name: String::new(), house: "The House".into(), silhouette: true }
    }
}

impl Settings {
    pub fn load() -> Settings {
        let mut s = Settings::default();
        let Some(text) = file("glider.cfg").and_then(|p| std::fs::read_to_string(p).ok()) else { return s };
        for l in text.lines() {
            let (k, v) = l.split_once(' ').unwrap_or((l, ""));
            match k {
                "sound" => s.sound = v != "off",
                "music" => s.music = v != "off",
                "name" => s.name = clean(v),
                "house" => s.house = v.to_string(),
                "view" => s.silhouette = v != "original",
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) {
        let Some(p) = file("glider.cfg") else { return };
        let on = |b: bool| if b { "on" } else { "off" };
        let view = if self.silhouette { "silhouette" } else { "original" };
        let text = format!("sound {}\nmusic {}\nname {}\nhouse {}\nview {view}\n", on(self.sound), on(self.music), clean(&self.name), self.house);
        let _ = std::fs::write(p, text);
    }
}
