//! Glider's sounds, from `snd.r` (8-bit unsigned mono at 22 kHz), mixed in
//! software. Glider played them through Patrick Buckland's SMS on four
//! channels; here effects get four voices, the room music and the bass
//! beat one each. A sound already playing starts again on its own voice
//! (the score odometer ticks every frame), else it takes a free voice or
//! the one furthest through its sound. Without an audio device, or with
//! `--mute`, every call does nothing.

use crate::game::snd;
use crate::mac::Sound;
use rodio::{MixerDeviceSink, Source};
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const RATE: u32 = 22_254;
const EFFECT_VOICES: usize = 4;
const MUSIC: usize = EFFECT_VOICES;
const BASS: usize = EFFECT_VOICES + 1;
const VOICES: usize = EFFECT_VOICES + 2;

#[derive(Clone, Copy, Default)]
struct Voice {
    sound: usize,
    /// Position in the sample, 16.16 fixed point.
    pos: u64,
    step: u64,
    len: u64,
}

impl Voice {
    fn playing(&self) -> bool {
        self.pos < self.len
    }
}

struct Mixer {
    bank: Arc<Vec<Option<Sound>>>,
    shared: Arc<Mutex<Vec<i32>>>,
    voices: [Voice; VOICES],
    poll: u32,
}

impl Mixer {
    fn start(&mut self, n: i32) {
        let i = n as usize;
        let Some(Some(s)) = self.bank.get(i) else { return };
        let v = match n {
            snd::MUSIC => MUSIC,
            snd::BASS => BASS,
            _ => {
                let own = (0..EFFECT_VOICES).find(|&c| self.voices[c].playing() && self.voices[c].sound == i);
                let free = (0..EFFECT_VOICES).find(|&c| !self.voices[c].playing());
                own.or(free).unwrap_or_else(|| (0..EFFECT_VOICES).max_by_key(|&c| self.voices[c].pos * 1000 / self.voices[c].len.max(1)).unwrap_or(0))
            }
        };
        self.voices[v] = Voice { sound: i, pos: 0, step: ((s.rate as u64) << 16) / RATE as u64, len: (s.data.len() as u64) << 16 };
    }
}

impl Iterator for Mixer {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.poll == 0 {
            self.poll = 64;
            let started: Vec<i32> = self.shared.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
            for s in started {
                self.start(s);
            }
        }
        self.poll -= 1;
        let mut mix: i32 = 0;
        for v in &mut self.voices {
            if v.playing() {
                if let Some(Some(s)) = self.bank.get(v.sound) {
                    mix += s.data[(v.pos >> 16) as usize] as i32 - 128;
                }
                v.pos += v.step;
            }
        }
        // four voices of +-128 at about 45% each keeps clear of clipping
        Some((mix as f32 / 128.0 * 0.45).clamp(-1.0, 1.0))
    }
}

impl Source for Mixer {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        NonZero::new(1).unwrap()
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        NonZero::new(RATE).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub struct Audio {
    // the sink must outlive the mixer playing on it
    sink: Option<MixerDeviceSink>,
    queue: Arc<Mutex<Vec<i32>>>,
    pub sound: bool,
    pub music: bool,
}

impl Audio {
    pub fn new(off: bool, bank: Vec<Option<Sound>>, sound: bool, music: bool) -> Audio {
        let queue = Arc::new(Mutex::new(Vec::new()));
        let sink = if off {
            None
        } else {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    let m = Mixer { bank: Arc::new(bank), shared: queue.clone(), voices: [Voice::default(); VOICES], poll: 0 };
                    s.mixer().add(m);
                    Some(s)
                }
                Err(e) => {
                    log::warn!("no audio: {e}");
                    None
                }
            }
        };
        Audio { sink, queue, sound, music }
    }

    pub fn available(&self) -> bool {
        self.sink.is_some()
    }

    /// Sound `n`, 1 to 30 as numbered in Glider. The music and the bass
    /// beat follow the music setting (DoTheMusic, DoTheBass), the rest the
    /// sound setting (DoTheSound).
    pub fn play(&mut self, n: i32) {
        let on = match n {
            snd::MUSIC => self.music && self.sound,
            snd::BASS => self.music,
            _ => self.sound,
        };
        if !on || self.sink.is_none() {
            return;
        }
        if let Ok(mut q) = self.queue.lock() {
            q.push(n);
        }
    }
}
