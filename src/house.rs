//! Glider 4 house files (`houseRec` in G-Globals.p), read as written by
//! THINK Pascal on a 68000: big-endian, strings as Pascal `string[n]`
//! padded to an even length, Booleans one byte.
//!
//! The houses that come with Glider are built into the game; any other
//! Glider 4 house file can be played from disk.

use crate::mac::{mac_roman, pstring};
use std::path::{Path, PathBuf};

pub const HOUSE_LEN: usize = 13830;
const HEADER: usize = 1270;
const ROOM: usize = 314;
pub const MAX_ROOMS: usize = 40;
pub const MAX_OBJECTS: usize = 16;

/// A QuickDraw rectangle, in SetRect's order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect { left, top, right, bottom }
    }

    /// `nullRect`, far off the screen.
    pub const NULL: Rect = Rect::new(-500, -500, -500, -500);

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn offset(&mut self, dx: i32, dy: i32) {
        self.left += dx;
        self.right += dx;
        self.top += dy;
        self.bottom += dy;
    }

    pub fn offset_by(mut self, dx: i32, dy: i32) -> Rect {
        self.offset(dx, dy);
        self
    }

    pub fn inset(&mut self, dx: i32, dy: i32) {
        self.left += dx;
        self.right -= dx;
        self.top += dy;
        self.bottom -= dy;
    }

    pub fn inset_by(mut self, dx: i32, dy: i32) -> Rect {
        self.inset(dx, dy);
        self
    }

    /// The same size, moved so its top left is at (x, y).
    pub fn at(self, x: i32, y: i32) -> Rect {
        self.offset_by(x - self.left, y - self.top)
    }

    pub fn empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// SectRect's answer: do they overlap by at least a pixel?
    pub fn sect(&self, o: &Rect) -> bool {
        self.left.max(o.left) < self.right.min(o.right) && self.top.max(o.top) < self.bottom.min(o.bottom)
    }

    /// UnionRect.
    pub fn union(&self, o: &Rect) -> Rect {
        if self.empty() {
            return *o;
        }
        if o.empty() {
            return *self;
        }
        Rect::new(self.left.min(o.left), self.top.min(o.top), self.right.max(o.right), self.bottom.max(o.bottom))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Object {
    pub kind: i32,
    pub bound: Rect,
    pub amount: i32,
    pub extra: i32,
    pub is_on: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Room {
    pub name: Vec<u8>,
    pub n_objects: i32,
    pub back_pict: i32,
    pub tile_order: [i32; 8],
    pub left_open: bool,
    pub right_open: bool,
    pub animate_kind: i32,
    pub animate_number: i32,
    pub animate_delay: i32,
    pub condition: i32,
    pub objects: [Object; MAX_OBJECTS],
}

#[derive(Clone, Debug)]
pub struct House {
    pub version: i32,
    pub n_rooms: i32,
    pub next_file: Vec<u8>,
    pub first_file: Vec<u8>,
    /// Rooms 1 to 40 at indexes 1 to 40; index 0 is unused, as in the Pascal.
    pub rooms: Vec<Room>,
}

fn be16(d: &[u8], at: usize) -> i32 {
    i16::from_be_bytes([d[at], d[at + 1]]) as i32
}

fn be32(d: &[u8], at: usize) -> i32 {
    i32::from_be_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

/// A Mac `Rect` as stored: top, left, bottom, right.
fn mac_rect(d: &[u8], at: usize) -> Rect {
    Rect::new(be16(d, at + 2), be16(d, at), be16(d, at + 6), be16(d, at + 4))
}

impl House {
    pub fn parse(d: &[u8]) -> Option<House> {
        if d.len() < HOUSE_LEN {
            return None;
        }
        let version = be16(d, 0);
        if version != 0x0100 {
            return None;
        }
        let mut rooms = vec![Room::default()];
        for r in 0..MAX_ROOMS {
            let o = HEADER + r * ROOM;
            let mut room = Room {
                name: pstring(&d[o..o + 26]),
                n_objects: be16(d, o + 26).clamp(0, MAX_OBJECTS as i32),
                back_pict: be16(d, o + 28),
                tile_order: [0; 8],
                left_open: d[o + 46] != 0,
                right_open: d[o + 47] != 0,
                animate_kind: be16(d, o + 48),
                animate_number: be16(d, o + 50),
                animate_delay: be32(d, o + 52),
                condition: be16(d, o + 56),
                objects: [Object::default(); MAX_OBJECTS],
            };
            for (i, t) in room.tile_order.iter_mut().enumerate() {
                *t = be16(d, o + 30 + i * 2).clamp(0, 7);
            }
            for (i, ob) in room.objects.iter_mut().enumerate() {
                let q = o + 58 + i * 16;
                *ob = Object { kind: be16(d, q), bound: mac_rect(d, q + 2), amount: be16(d, q + 10), extra: be16(d, q + 12), is_on: d[q + 14] != 0 };
            }
            rooms.push(room);
        }
        Some(House {
            version,
            n_rooms: be16(d, 2).clamp(0, MAX_ROOMS as i32),
            next_file: pstring(&d[1202..1236]),
            first_file: pstring(&d[1236..1270]),
            rooms,
        })
    }
}

/// Does a house name point anywhere? The Pascal treats '' and 'nil' as no.
pub fn names_a_file(n: &[u8]) -> bool {
    !n.is_empty() && n != b"nil"
}

macro_rules! houses {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../Houses/", $name)) as &[u8])),*]
    };
}

/// The houses in the Glider 4 repository, by file name.
pub const BUILT_IN: &[(&str, &[u8])] = houses![
    "The House",
    "The House 2",
    "Combo House",
    "Combo House 2",
    "Combo House 3",
    "Combo House 4",
    "Glass House",
    "Glass House 2",
    "Hands-off House",
    "House Full of Stuff",
    "House Full of Stuff 2",
    "House Full of Stuff Next Door",
    "House Full of Stuff Next Door 2",
    "House of Doom",
    "House of the Rising Sun",
    "House of the Rising Sun 1",
    "House of the Rising Sun 2",
    "House of the Rising Sun 3",
    "Lumpy's Home",
    "Mad House",
    "Mad House 2",
    "Usher House",
];

/// Where houses come from: the built-in ones, and a folder for others (the
/// folder of a house file opened from disk, so its next files are found
/// beside it, as on the Mac).
#[derive(Clone, Default)]
pub struct Library {
    pub dir: Option<PathBuf>,
}

impl Library {
    pub fn load(&self, name: &[u8]) -> Option<House> {
        let text = mac_roman(name);
        if let Some(dir) = &self.dir {
            let p = dir.join(&text);
            if let Ok(d) = std::fs::read(&p) {
                return House::parse(&d);
            }
        }
        BUILT_IN.iter().find(|(n, _)| n.as_bytes() == name || *n == text).and_then(|(_, d)| House::parse(d))
    }

    /// A house file from disk; the library then looks beside it first.
    pub fn open(path: &Path) -> Option<(Library, Vec<u8>, House)> {
        let d = std::fs::read(path).ok()?;
        let h = House::parse(&d)?;
        let name = path.file_name()?.to_string_lossy().into_owned();
        let lib = Library { dir: path.parent().map(Path::to_path_buf) };
        // file names are UTF-8 here; house names inside are MacRoman, and
        // only the ASCII ones round-trip, which is all the shipped houses use
        Some((lib, name.into_bytes(), h))
    }
}

/// The built-in houses a game starts in: those that are not the second
/// part of another.
pub fn starting_houses() -> Vec<&'static str> {
    BUILT_IN
        .iter()
        .filter(|(n, d)| House::parse(d).is_some_and(|h| !names_a_file(&h.first_file) || h.first_file == n.as_bytes()))
        .map(|(n, _)| *n)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_house_reads() {
        for (n, d) in BUILT_IN {
            let h = House::parse(d).unwrap_or_else(|| panic!("{n}"));
            assert!(h.n_rooms > 0, "{n}");
        }
    }

    #[test]
    fn the_house_starts_with_welcome() {
        let h = Library::default().load(b"The House").unwrap();
        assert_eq!(h.rooms[1].name, b"Welcome...");
        assert!(!h.rooms[1].left_open && h.rooms[1].right_open);
        assert_eq!(h.rooms[1].objects[1].bound, Rect::new(186, 223, 356, 232));
        assert_eq!(h.next_file, b"The House 2");
    }
}
