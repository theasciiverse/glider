//! Everything Glider draws and plays, decoded at start-up from the files in
//! John Calhoun's repository: `Color Art` (objects, masks, backgrounds,
//! stairs, the farm), `Glider.r` (title screen, help pages, fonts) and
//! `snd.r` (the sounds).

use crate::mac::{self, Font, Image, Resources, Sound};
use std::collections::HashMap;

const COLOR_ART: &[u8] = include_bytes!("../Glider_405/Color Art.bin");
const GLIDER_R: &[u8] = include_bytes!("../Glider_405/Glider.r");
const SND_R: &[u8] = include_bytes!("../Glider_Sound_Files/snd.r");

pub struct Assets {
    /// PICT 128: every sprite.
    pub objects: Image,
    /// PICT 129: the masks, black where a sprite is drawn.
    pub mask: Vec<bool>,
    pub backgrounds: HashMap<i32, Image>,
    /// PICT 130: the farm of the closing animation.
    pub farm: Image,
    pub stair_up: Image,
    pub stair_down: Image,
    /// PICT 210: the colour title screen.
    pub idle: Image,
    /// PICT 1000 and the help pages 1001 to 1007.
    pub help: Vec<Image>,
    /// FONT 2060 (family 16 at 12 points), the game's small text.
    pub font: Font,
    /// Sounds 1 to 30, from `snd ` 1001 to 1030.
    pub sounds: Vec<Option<Sound>>,
    /// STR# 131: the title screen's messages.
    pub demo_strings: Vec<String>,
}

fn pict(r: &Resources, id: i16) -> Image {
    r.get(&(*b"PICT", id))
        .and_then(|v| match mac::pict(v) {
            Ok(i) => Some(i),
            Err(e) => {
                log::warn!("PICT {id}: {e}");
                None
            }
        })
        .unwrap_or_else(|| Image::new(512, 342, [255; 3]))
}

fn strings(r: &Resources, id: i16) -> Vec<String> {
    let Some(v) = r.get(&(*b"STR#", id)) else { return Vec::new() };
    let n = i16::from_be_bytes([v[0], v[1]]).max(0) as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = 2;
    for _ in 0..n {
        let Some(&l) = v.get(p) else { break };
        let end = (p + 1 + l as usize).min(v.len());
        out.push(mac::mac_roman(&v[p + 1..end]));
        p = end;
    }
    out
}

impl Assets {
    pub fn load() -> Assets {
        let art = mac::resource_fork(mac::macbinary_rsrc(COLOR_ART));
        let app = mac::derez(GLIDER_R);
        let snd = mac::derez(SND_R);
        let mask_img = pict(&art, 129);
        let mask = mask_img.px.iter().map(|c| c[0] < 128).collect();
        let backgrounds = (200..=209).map(|id| (id, pict(&art, id as i16))).collect();
        let font = |id: i16| app.get(&(*b"FONT", id)).and_then(|v| Font::parse(v)).expect("Glider.r carries its fonts");
        Assets {
            objects: pict(&art, 128),
            mask,
            backgrounds,
            farm: pict(&art, 130),
            stair_up: pict(&art, 198),
            stair_down: pict(&art, 199),
            idle: pict(&app, 210),
            help: (1000..=1007).map(|id| pict(&app, id)).collect(),
            font: font(2060),
            sounds: (0..=30).map(|n| if n == 0 { None } else { snd.get(&(*b"snd ", 1000 + n)).and_then(|v| mac::snd(v)) }).collect(),
            demo_strings: strings(&app, 131),
        }
    }

    pub fn mask_at(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.objects.w && y < self.objects.h && self.mask[(y * self.objects.w + x) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_picture_and_sound_decodes() {
        let art = mac::resource_fork(mac::macbinary_rsrc(COLOR_ART));
        for ((t, id), v) in &art {
            if t == b"PICT" {
                let i = mac::pict(v).unwrap_or_else(|e| panic!("art PICT {id}: {e}"));
                assert!(i.w > 0 && i.h > 0);
            }
        }
        let app = mac::derez(GLIDER_R);
        for id in [140, 141, 200, 210, 228, 229, 256, 1000, 1001, 1007] {
            mac::pict(&app[&(*b"PICT", id)]).unwrap_or_else(|e| panic!("Glider.r PICT {id}: {e}"));
        }
        let a = Assets::load();
        assert_eq!((a.objects.w, a.objects.h), (512, 342));
        assert_eq!(a.sounds.iter().flatten().count(), 30);
        assert!(a.sounds.iter().flatten().all(|s| s.rate > 11_000 && !s.data.is_empty()));
    }
}
