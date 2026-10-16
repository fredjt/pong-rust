/// Audio subsystem — procedural sound synthesis (section 10)
/// All sounds are generated as WAV bytes at startup and loaded via macroquad.
/// Zero asset files; toggle with `M` key.

use macroquad::audio::{load_sound_from_bytes, play_sound_once, stop_sound, Sound};

const SR: u32 = 44_100;

// ── WAV builder helpers ───────────────────────────────────────────────

fn wav_header(pcm_len: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + pcm_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());           // chunk size
    out.extend_from_slice(&1u16.to_le_bytes());             // PCM
    out.extend_from_slice(&2u16.to_le_bytes());             // stereo
    out.extend_from_slice(&SR.to_le_bytes());               // sample rate
    out.extend_from_slice(&(SR * 4).to_le_bytes());         // byte rate
    out.extend_from_slice(&4u16.to_le_bytes());             // block align
    out.extend_from_slice(&16u16.to_le_bytes());            // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(pcm_len as u32).to_le_bytes());
    out
}

/// Build a square-wave sound and return WAV bytes.
fn blip(start_hz: f32, end_hz: f32, dur_s: f32, vol: f32) -> Vec<u8> {
    let n = (dur_s * SR as f32) as usize;
    let mut pcm: Vec<i16> = Vec::with_capacity(n);
    let fade_in_samples = (0.003 * SR as f32) as usize; // 3 ms
    let fade_out_samples = (0.010 * SR as f32) as usize; // 10 ms

    for i in 0..n {
        let t = i as f32 / SR as f32;
        // Linear frequency sweep
        let f = start_hz + (end_hz - start_hz) * (i as f32 / n as f32);
        // Square wave via phase accumulator
        let phase = ((t * f) % 1.0).max(0.0);
        let sample = if phase < 0.5 { 1 } else { -1 };
        // Amplitude envelope
        let mut amp = 1.0_f32;
        if i < fade_in_samples {
            amp = i as f32 / fade_in_samples as f32;
        }
        if i >= n - fade_out_samples {
            let idx = i - (n - fade_out_samples);
            amp = 1.0 - idx as f32 / fade_out_samples as f32;
        }
        pcm.push((sample as f32 * vol * amp * 32767.0).clamp(-32768.0, 32767.0) as i16);
    }

    let pcm_bytes: Vec<u8> = pcm.iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    let mut out = wav_header(pcm_bytes.len());
    out.extend_from_slice(&pcm_bytes);
    out
}

// ── Sound definitions (section 10.2) ──────────────────────────────────

fn paddle_sound() -> Vec<u8> { blip(440.0, 440.0, 0.045, 0.35) }
fn wall_sound()   -> Vec<u8> { blip(320.0, 320.0, 0.035, 0.28) }
fn score_sound()  -> Vec<u8> { blip(240.0, 120.0, 0.16, 0.40) }
fn win_sound()    -> Vec<u8> {
    // Three notes back-to-back: 523, 659, 784 Hz (~0.12 s each)
    let mut out: Vec<u8> = Vec::new();
    for (hz, dur) in [(523.0, 0.12), (659.0, 0.12), (784.0, 0.12)] {
        out.extend_from_slice(&blip(hz, hz, dur, 0.40));
    }
    out
}
fn select_sound() -> Vec<u8> { blip(660.0, 660.0, 0.030, 0.30) }

// ── Audio struct & playback ───────────────────────────────────────────

/// Playback helper — retrigger by stopping then playing (design:10.3).
pub fn play_sound(s: &Sound) {
    stop_sound(s);
    play_sound_once(s);
}

pub struct Audio {
    pub enabled: bool,
    paddle: Sound,
    wall: Sound,
    score: Sound,
    win: Sound,
    select: Sound,
}

impl Audio {
    /// Load all sounds from in-memory WAV bytes.  Returns sound-on by default.
    pub async fn new() -> Self {
        let paddle = load_sound_from_bytes(&paddle_sound()).await.unwrap();
        let wall   = load_sound_from_bytes(&wall_sound()).await.unwrap();
        let score  = load_sound_from_bytes(&score_sound()).await.unwrap();
        let win    = load_sound_from_bytes(&win_sound()).await.unwrap();
        let select = load_sound_from_bytes(&select_sound()).await.unwrap();

        Audio {
            enabled: true,
            paddle, wall, score, win, select,
        }
    }

    pub fn play_paddle(&self) { if self.enabled { play_sound(&self.paddle); } }
    pub fn play_wall(&self)   { if self.enabled { play_sound(&self.wall); } }
    pub fn play_score(&self)  { if self.enabled { play_sound(&self.score); } }
    pub fn play_win(&self)    { if self.enabled { play_sound(&self.win); } }
    pub fn play_select(&self) { if self.enabled { play_sound(&self.select); } }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}
