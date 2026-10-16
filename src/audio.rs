//! Audio subsystem — procedural sound synthesis (design section 10).
//!
//! Every sound is synthesised into WAV bytes at startup and handed to
//! `macroquad::audio::load_sound_from_bytes`. No asset files, no extra crates.
//! Sound starts ON and is toggled with `M`.
//!
//! Two layout rules decide whether the loader accepts a synthesised sound:
//!  * one sound == exactly one RIFF/WAVE container with one `data` chunk —
//!    concatenating finished WAV files yields trailing containers that readers
//!    walk past, so only the first note survives (see `win_sound`);
//!  * the header declares 2-channel / 16-bit PCM, so every mono sample must be
//!    written twice (L, R) — see `to_wav`. Writing one i16 per sample under a
//!    stereo header makes readers count half as many frames and every sound
//!    plays at double pitch/half length.

use macroquad::audio::{load_sound_from_bytes, play_sound_once, stop_sound, Sound};

const SR: u32 = 44_100;
const CHANNELS: u16 = 2;
const BITS: u16 = 16;
/// Bytes per interleaved frame: 2 channels x 16-bit samples.
const BYTES_PER_FRAME: usize = (CHANNELS as usize) * (BITS as usize / 8);

const FADE_IN_S: f32 = 0.003; // 3 ms — kills the leading-edge click
const FADE_OUT_S: f32 = 0.010; // 10 ms — same for the tail

// ── WAV builder helpers ───────────────────────────────────────────────

/// Build the 44-byte RIFF/WAVE header describing `pcm_len` bytes of PCM.
fn wav_header(pcm_len: usize) -> Vec<u8> {
    let byte_rate = SR * CHANNELS as u32 * (BITS as u32 / 8); // 176_400
    let block_align = BYTES_PER_FRAME as u16; // 4

    let mut out: Vec<u8> = Vec::with_capacity(44 + pcm_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + pcm_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // 1 == PCM
    out.extend_from_slice(&CHANNELS.to_le_bytes());
    out.extend_from_slice(&SR.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&BITS.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(pcm_len as u32).to_le_bytes());
    out
}

/// Render one square wave (50 % duty) as mono `i16` samples — *not* wrapped in a
/// WAV container, so several tones can be concatenated into one sound.
///
/// `start_hz == end_hz` gives a fixed pitch; anything else sweeps linearly.
fn render_pcm(start_hz: f32, end_hz: f32, dur_s: f32, vol: f32) -> Vec<i16> {
    let n = (dur_s.max(0.0) * SR as f32) as usize;
    if n == 0 {
        return Vec::new();
    }
    let mut pcm: Vec<i16> = Vec::with_capacity(n);

    // Fades may never be longer than the buffer they sit in: `n - fade_out` with
    // `fade_out > n` underflows, which panics debug builds outright.
    let fade_in = ((FADE_IN_S * SR as f32) as usize).min(n / 2);
    let fade_out = ((FADE_OUT_S * SR as f32) as usize).min(n / 2).max(1);
    let tail_start = n - fade_out; // first sample of the fade-out ramp

    // Total sweep duration in seconds (n >= 1 here, so this is never 0).
    let total_s = n as f32 / SR as f32;

    for i in 0..n {
        let t = i as f32 / SR as f32;

        // A square wave's zero crossings are spaced by the *integral* of its
        // instantaneous frequency, not by `t * f(t)`:
        //     f(t)     = start + (end - start) * t / T
        //     phase(t) = ∫f  = start * t + (end - start) * t² / (2T)
        // Using `t * f(t)` instead doubles the sweep term, so a 240 -> 120 Hz
        // glide lands on ~0 Hz at its end rather than 120 Hz. For a fixed pitch
        // (start == end) both forms agree, which is why only swept sounds broke.
        let phase = (start_hz * t + (end_hz - start_hz) * t * t / (2.0 * total_s)).rem_euclid(1.0);
        let square = if phase < 0.5 { 1.0 } else { -1.0 };

        // Linear fade-in / fade-out envelope.
        let mut amp = 1.0_f32;
        if fade_in > 0 && i < fade_in {
            amp = i as f32 / fade_in as f32;
        }
        if i >= tail_start {
            amp = amp.min(1.0 - (i - tail_start) as f32 / fade_out as f32);
        }

        pcm.push((square * vol * amp * 32767.0).clamp(-32768.0, 32767.0) as i16);
    }

    pcm
}

/// Wrap mono samples in a single stereo WAV container (design 10.1 step 3: each
/// sample's two bytes are pushed twice to form the L and R channel).
fn to_wav(samples: &[i16]) -> Vec<u8> {
    let mut pcm_bytes: Vec<u8> = Vec::with_capacity(samples.len() * BYTES_PER_FRAME);
    for s in samples {
        let bytes = s.to_le_bytes();
        pcm_bytes.extend_from_slice(&bytes); // left
        pcm_bytes.extend_from_slice(&bytes); // right (mono duplicated)
    }
    let mut out = wav_header(pcm_bytes.len());
    out.extend_from_slice(&pcm_bytes);
    out
}

/// Build one square-wave sound and return its WAV bytes.
fn blip(start_hz: f32, end_hz: f32, dur_s: f32, vol: f32) -> Vec<u8> {
    to_wav(&render_pcm(start_hz, end_hz, dur_s, vol))
}

// ── Sound definitions (section 10.2) ──────────────────────────────────

fn paddle_sound() -> Vec<u8> {
    blip(440.0, 440.0, 0.045, 0.35)
}
fn wall_sound() -> Vec<u8> {
    blip(320.0, 320.0, 0.035, 0.28)
}
fn score_sound() -> Vec<u8> {
    blip(240.0, 120.0, 0.16, 0.40)
}
fn select_sound() -> Vec<u8> {
    blip(660.0, 660.0, 0.030, 0.30)
}

/// Win fanfare: three notes back to back (523, 659, 784 Hz, ~0.12 s each).
///
/// The notes are concatenated as raw PCM and only then wrapped in a header, so
/// the result is one valid WAV file. Concatenating three `blip()` outputs would
/// emit three RIFF containers, and readers stop at the end of the first `data`
/// chunk — i.e. a single 0.12 s note instead of the fanfare.
fn win_sound() -> Vec<u8> {
    const NOTES: [(f32, f32); 3] = [(523.0, 0.12), (659.0, 0.12), (784.0, 0.12)];
    const VOL: f32 = 0.40;

    let mut pcm: Vec<i16> = Vec::new();
    for (hz, dur) in NOTES {
        pcm.extend_from_slice(&render_pcm(hz, hz, dur, VOL));
    }
    to_wav(&pcm)
}

// ── Audio struct & playback ───────────────────────────────────────────

/// Playback helper — retrigger by stopping then playing (design:10.3).
pub fn play_sound(s: &Sound) {
    stop_sound(s);
    play_sound_once(s);
}

/// Play a slot that may not have loaded; silent no-op when it is `None`.
fn play_slot(slot: &Option<Sound>) {
    if let Some(sound) = slot {
        play_sound(sound);
    }
}

/// Synthesise one sound and load it, reporting failure as `None` instead of
/// unwrapping. A failed decode leaves that sound silent; the game still runs.
async fn load_slot(name: &str, wav: &[u8]) -> Option<Sound> {
    match load_sound_from_bytes(wav).await {
        Ok(sound) => Some(sound),
        Err(err) => {
            eprintln!("[audio] could not load '{name}' ({err:?}) — continuing without it");
            None
        }
    }
}

/// The five game sounds plus the master switch.
///
/// Each sound is an `Option<Sound>`: a sound that failed to decode stays `None`
/// and its `play_*` method is a no-op, so a broken decoder degrades to silence
/// instead of panicking before the first frame is drawn.
pub struct Audio {
    pub enabled: bool,
    paddle: Option<Sound>,
    wall: Option<Sound>,
    score: Option<Sound>,
    win: Option<Sound>,
    select: Option<Sound>,
}

impl Audio {
    /// Synthesise every sound and load it from in-memory WAV bytes (design 10.3).
    ///
    /// Sound is ON when all five sounds loaded; if any failed we start muted,
    /// having logged which ones, and never panic. (When macroquad cannot open an
    /// audio device at all it aborts while building its context, before this
    /// function is ever reached — that path is outside this crate.)
    pub async fn new() -> Self {
        let paddle = load_slot("paddle", &paddle_sound()).await;
        let wall = load_slot("wall", &wall_sound()).await;
        let score = load_slot("score", &score_sound()).await;
        let win = load_slot("win", &win_sound()).await;
        let select = load_slot("select", &select_sound()).await;

        let all_loaded = paddle.is_some()
            && wall.is_some()
            && score.is_some()
            && win.is_some()
            && select.is_some();
        if !all_loaded {
            eprintln!("[audio] starting muted — press M to retry the sounds that did load");
        }

        Audio {
            enabled: all_loaded,
            paddle,
            wall,
            score,
            win,
            select,
        }
    }

    pub fn play_paddle(&self) {
        if self.enabled {
            play_slot(&self.paddle);
        }
    }
    pub fn play_wall(&self) {
        if self.enabled {
            play_slot(&self.wall);
        }
    }
    pub fn play_score(&self) {
        if self.enabled {
            play_slot(&self.score);
        }
    }
    pub fn play_win(&self) {
        if self.enabled {
            play_slot(&self.win);
        }
    }
    pub fn play_select(&self) {
        if self.enabled {
            play_slot(&self.select);
        }
    }

    /// Toggle the master switch. Stays off when nothing loaded at all, so the
    /// HUD cannot claim `SOUND: ON` while every slot is empty.
    pub fn toggle(&mut self) {
        let anything_loaded = self.paddle.is_some()
            || self.wall.is_some()
            || self.score.is_some()
            || self.win.is_some()
            || self.select.is_some();
        if anything_loaded {
            self.enabled = !self.enabled;
        } else {
            self.enabled = false;
        }
    }
}

// ── Unit tests (synthesis only — no audio device needed) ──────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A parsed view of a synthesised sound, for asserting on the container.
    struct WavFile {
        containers: usize,
        channels: u16,
        sample_rate: u32,
        block_align: u16,
        bits: u16,
        data_len: usize,
        bytes_after_data: usize,
    }

    fn parse(bytes: &[u8]) -> WavFile {
        assert!(bytes.len() >= 44, "a WAV file is at least 44 bytes");
        let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        let u32_at =
            |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);

        assert_eq!(&bytes[0..4], b"RIFF", "RIFF magic");
        assert_eq!(&bytes[8..12], b"WAVE", "WAVE magic");
        assert_eq!(&bytes[12..16], b"fmt ", "fmt chunk");
        assert_eq!(&bytes[36..40], b"data", "data chunk");

        let data_len = u32_at(40) as usize;
        WavFile {
            containers: bytes.windows(4).filter(|w| *w == b"RIFF").count(),
            channels: u16_at(22),
            sample_rate: u32_at(24),
            block_align: u16_at(32),
            bits: u16_at(34),
            data_len,
            bytes_after_data: bytes.len().saturating_sub(44 + data_len),
        }
    }

    /// Left-channel samples (i.e. the mono waveform) of a synthesised sound.
    fn mono(bytes: &[u8]) -> Vec<i16> {
        bytes[44..]
            .chunks(BYTES_PER_FRAME)
            .map(|frame| i16::from_le_bytes([frame[0], frame[1]]))
            .collect()
    }

    /// Sign changes of the waveform in `[from, to)` — one full cycle of a square
    /// wave produces exactly two of them (rising and falling edge).
    fn edges(samples: &[i16], from: usize, to: usize) -> f32 {
        samples[from..to]
            .windows(2)
            .filter(|w| (w[0] > 0) != (w[1] > 0))
            .count() as f32
    }

    /// Cycles a sweep should have completed between two times: the integral of
    /// `f(t) = start + (end - start) * t / T`, i.e.
    /// `start * Δt + (end - start) * (t1² - t0²) / (2T)`.
    fn cycles_elapsed(start_hz: f32, end_hz: f32, total_s: f32, t0: f32, t1: f32) -> f32 {
        start_hz * (t1 - t0) + (end_hz - start_hz) * (t1 * t1 - t0 * t0) / (2.0 * total_s)
    }

    /// Assert the rendered tone contains exactly the number of zero crossings its
    /// frequency law promises between `from` and `to` (+/- 1 edge, since edges are
    /// whole numbers). Pins down pitch, sweep shape and duration at once. `samples`
    /// must start at the tone's own t = 0, so a single note of a multi-note sound
    /// is passed as its own slice.
    fn assert_cycle_count(
        samples: &[i16],
        label: &str,
        start_hz: f32,
        end_hz: f32,
        from: usize,
        to: usize,
    ) {
        let total_s = samples.len() as f32 / SR as f32;
        let expected = 2.0
            * cycles_elapsed(
                start_hz,
                end_hz,
                total_s,
                from as f32 / SR as f32,
                to as f32 / SR as f32,
            );
        let measured = edges(samples, from, to);
        assert!(
            (measured - expected).abs() <= 1.5,
            "{label}: {measured} zero crossings between samples {from}..{to}, \
             but the frequency law promises {expected:.2}"
        );
    }

    /// Samples of a sound excluding both fade ramps, i.e. the steady part whose
    /// zero crossings are meaningful.
    fn steady_window(samples: &[i16]) -> (usize, usize) {
        let fade_in = ((FADE_IN_S * SR as f32) as usize).min(samples.len() / 2);
        let fade_out = ((FADE_OUT_S * SR as f32) as usize)
            .min(samples.len() / 2)
            .max(1);
        (fade_in + 1, samples.len() - fade_out)
    }

    #[test]
    fn wav_header_describes_44k_stereo_pcm() {
        let wav = blip(440.0, 440.0, 0.05, 0.5);
        let file = parse(&wav);
        assert_eq!(file.channels, 2);
        assert_eq!(file.sample_rate, SR);
        assert_eq!(file.bits, 16);
        assert_eq!(file.block_align, BYTES_PER_FRAME as u16);
        assert_eq!(
            file.bytes_after_data, 0,
            "no trailing junk after the data chunk"
        );
    }

    #[test]
    fn blip_frame_count_matches_its_duration() {
        // Regression: the payload used to hold one i16 per sample while the
        // header declared stereo, so readers saw half the frames and every
        // sound played at double pitch / half length.
        let dur = 0.045;
        let wav = blip(440.0, 440.0, dur, 0.35);
        let file = parse(&wav);
        let expected_frames = (dur * SR as f32) as usize;

        assert_eq!(file.containers, 1, "one sound is one RIFF container");
        assert_eq!(file.data_len, expected_frames * BYTES_PER_FRAME);
        assert_eq!(mono(&wav).len(), expected_frames);

        let measured_s = file.data_len as f32 / (file.sample_rate as f32 * file.block_align as f32);
        assert!(
            (measured_s - dur).abs() < 1.0 / SR as f32,
            "header advertises {measured_s} s but the sound is {dur} s"
        );
    }

    #[test]
    fn win_sound_is_one_container_holding_all_three_notes() {
        let wav = win_sound();
        let file = parse(&wav);
        assert_eq!(
            file.containers, 1,
            "concatenated WAVs leave trailing RIFF chunks"
        );
        assert_eq!(file.bytes_after_data, 0);

        let samples = mono(&wav);
        let note_frames = (0.12 * SR as f32) as usize;
        assert_eq!(
            samples.len(),
            3 * note_frames,
            "all three notes are present"
        );

        // Each third must carry its own pitch: 523, then 659, then 784 Hz. Notes
        // are sliced out because each restarts its own phase and envelope.
        let fade_in = (FADE_IN_S * SR as f32) as usize;
        let fade_out = (FADE_OUT_S * SR as f32) as usize;
        for (i, hz) in [523.0_f32, 659.0, 784.0].iter().enumerate() {
            let note = &samples[i * note_frames..(i + 1) * note_frames];
            assert_cycle_count(
                note,
                &format!("win note {}", i + 1),
                *hz,
                *hz,
                fade_in + 1,
                note_frames - fade_out,
            );
        }
    }

    #[test]
    fn swept_tone_follows_the_frequency_integral() {
        // 240 -> 120 Hz over 0.16 s. The old `(t * f(t)) % 1.0` phase doubled the
        // sweep term, so the tone slid down to ~0 Hz instead of 120 Hz — which is
        // what turned the score sound into a dull thud.
        let samples = mono(&score_sound());
        let n = samples.len();
        let (from, to) = steady_window(&samples);

        // Both halves separately: the edge counts only line up if the phase really
        // is the integral of the sweep (the old `t * f(t)` law promised ~53 edges
        // but produced ~37).
        assert_cycle_count(&samples, "sweep first half", 240.0, 120.0, from, n / 2);
        assert_cycle_count(&samples, "sweep second half", 240.0, 120.0, n / 2, to);

        // Readable companion check: a descending sweep has clearly more crossings
        // in its first half than in its second.
        let early = edges(&samples, from, n / 2);
        let late = edges(&samples, n / 2, to);
        assert!(
            early > late * 1.2,
            "sweep must descend: {early} crossings in the first half vs {late} in the second"
        );
    }

    #[test]
    fn fixed_pitch_tones_hold_their_frequency() {
        for (wav, hz) in [
            (paddle_sound(), 440.0),
            (wall_sound(), 320.0),
            (select_sound(), 660.0),
        ] {
            let samples = mono(&wav);
            let (from, to) = steady_window(&samples);
            assert_cycle_count(&samples, "tone", hz, hz, from, to);
        }
    }

    #[test]
    fn envelope_is_silent_at_the_edges_and_scaled_by_volume() {
        let vol = 0.35_f32;
        let samples = mono(&paddle_sound());
        assert!(
            samples[0].abs() < 64,
            "fade-in must start near silence, got {}",
            samples[0]
        );
        assert!(
            samples[samples.len() - 1].abs() < 64,
            "fade-out must end near silence, got {}",
            samples[samples.len() - 1]
        );

        let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap() as f32;
        let expected = vol * 32767.0;
        assert!(
            (peak - expected).abs() < expected * 0.05,
            "peak {peak} should be ~{expected} for volume {vol}"
        );
    }

    #[test]
    fn degenerate_durations_produce_valid_silent_wavs() {
        // Regression: the fade-out ramp computed `n - fade_out`, which underflowed
        // (panicking debug builds) for any sound shorter than the 10 ms fade.
        for dur in [0.0, 0.000_05, 0.000_5, 0.002] {
            let wav = blip(440.0, 440.0, dur, 1.0);
            let file = parse(&wav);
            assert_eq!(file.containers, 1);
            assert_eq!(file.bytes_after_data, 0);
            for s in mono(&wav) {
                assert!(
                    f32::from(s).is_finite(),
                    "sample must stay finite at dur {dur}"
                );
            }
        }
    }
}
