# PONG // RUST — Build Notes

## Binary Size
- Release binary: 939 KB (`target/release/pong-rust`)
- Well under the 12 MB target
- Was 767 KB before macroquad's `audio` feature was enabled; the ~170 KB delta is quad-snd / miniaudio

## Dependencies (ldd)
```
linux-vdso.so.1
libasound.so.2    — ALSA, pulled in by macroquad's `audio` feature (miniaudio)
libgcc_s.so.1     — GCC runtime (C++ stdlib fallback)
libm.so.6         — C math library
libc.so.6         — C standard library
ld-linux-x86-64   — dynamic linker
```
Only base system libraries — nothing bundled, no Python, no engine runtime. `libasound`
appears because real audio playback is now compiled in; it ships with the distro on every
Linux target.

## Audio
Full procedural synthesis (section 10). WAV bytes generated at startup, loaded via `load_sound_from_bytes`, played with `stop_sound` + `play_sound_once` (retriggering with `play_sound` alone restarts an already-playing tone and loses the staccato). Toggle with `M`. Sound is ON by default.

Two WAV layout rules are load-bearing, and both were bugs in the first draft:

1. **One sound = one RIFF container.** Multi-note sounds concatenate *raw PCM* and wrap it once (`win_sound`). Concatenating finished WAV files leaves trailing containers readers never reach, so only the first note plays.
2. **The header declares 2-channel / 16-bit PCM, so every mono sample is written twice** (L then R — `to_wav`). Emitting one i16 per frame under a stereo header makes readers count half as many frames: everything plays at double pitch and half length.

A swept square wave uses the **integral** of frequency (`start*t + (end-start)*t*t/(2T)`), not `t * f(t)`, which doubles the sweep term. Fixed-pitch tones are identical either way, which is why only the score sound was off-key.

Failure handling: each sound is an `Option<Sound>`. A decode failure logs and leaves that sound silent (audio starts muted if any failed) instead of panicking before the first frame. Note this does not cover a machine with no audio device at all — macroquad aborts while building its window context, before `Audio::new` runs.

## Events
`step(&mut Game, left_dir, right_dir, dt) -> Vec<Event>` returns **every** event raised in that frame, oldest first. A frame is split into sub-steps (`h = dt / ceil(dt / SUBSTEP_DT)`), so several distinct collisions can land inside one call; returning only the last event drops the earlier ones and their sounds never play. `SUBSTEP_DT` is a *target maximum* sub-step, not a step size — advancing physics by it directly desynchronises simulated time from real time (the ball runs fast at high refresh rates).

## Cold Start
Binary starts instantly (no sound loading in the loop — all sounds are pre-loaded).

## Cargo.toml Dependencies
Exactly one dependency: `macroquad = { version = "0.4", features = ["audio"] }`

## Unit Tests
All 23 tests pass without opening a window (no audio device needed):
- `audio::blip_frame_count_matches_its_duration`
- `audio::degenerate_durations_produce_valid_silent_wavs`
- `audio::envelope_is_silent_at_the_edges_and_scaled_by_volume`
- `audio::fixed_pitch_tones_hold_their_frequency`
- `audio::swept_tone_follows_the_frequency_integral`
- `audio::wav_header_describes_44k_stereo_pcm`
- `audio::win_sound_is_one_container_holding_all_three_notes`
- `game::a_quiet_frame_reports_no_events`
- `game::a_single_collision_yields_a_single_event`
- `game::ai_moves_toward_ball`
- `game::ball_bounces_off_bottom_wall`
- `game::ball_bounces_off_top_wall`
- `game::ball_cannot_tunnel_through_paddle`
- `game::edge_hit_produces_steep_angle`
- `game::every_event_in_a_frame_is_reported`
- `game::match_ends_at_win_score`
- `game::paddle_hit_flips_direction_and_speeds_up`
- `game::paddle_is_clamped_to_court`
- `game::prediction_reflects_off_walls`
- `game::rng_is_deterministic_and_in_range`
- `game::scoring_awards_point_and_serves_against_loser`
- `game::start_match_serves_toward_previous_loser`
- `game::step_advances_exactly_dt`

## Cross-Compilation
The same source compiles for Windows (`x86_64-pc-windows-gnu`) and macOS unchanged.
Web (WASM) target is also supported by macroquad without code changes.

## Formatting
`cargo build`, `cargo test` (23 tests) and `cargo clippy --all-targets` (no warnings) are clean.

The repo is **not** rustfmt-clean at its base commit: `cargo fmt --check` reports 12 pre-existing
hunks in `game.rs` (long `assert!` calls in the tests, a chained `let speed = ...`) and 13 in
`main.rs` (`mod` order plus wrapped `draw_*` calls). None were introduced here — run
`rustfmt --edition 2021 --check src/game.rs | grep -c 'Diff in'` and you get the same count as
the base commit. Formatting the whole crate is a separate, churn-heavy change; keep it out of
behaviour changes like this one.
