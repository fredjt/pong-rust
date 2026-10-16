# PONG // RUST — Build Notes

## Binary Size
- Release binary: 767 KB (`target/release/pong-rust`)
- Well under the 12 MB target

## Dependencies (ldd)
```
linux-vdso.so.1
libgcc_s.so.1     — GCC runtime (C++ stdlib fallback)
libm.so.6         — C math library
libc.so.6         — C standard library
ld-linux-x86-64   — dynamic linker
```
Only base system libraries. No bundled runtime, no Python, no engine runtime, no audio library.

## Audio
Full procedural synthesis (section 10). WAV bytes generated at startup, loaded via `load_sound_from_bytes`, played with `play_sound_once`. Toggle with `M`. Sound is ON by default.

## Cold Start
Binary starts instantly (no sound loading in the loop — all sounds are pre-loaded).

## Cargo.toml Dependencies
Exactly one dependency: `macroquad = { version = "0.4", features = ["audio"] }`

## Unit Tests
All 11 tests pass without opening a window:
- ball_bounces_off_top_wall
- ball_bounces_off_bottom_wall
- paddle_hit_flips_direction_and_speeds_up
- edge_hit_produces_steep_angle
- paddle_is_clamped_to_court
- ball_cannot_tunnel_through_paddle
- scoring_awards_point_and_serves_against_loser
- match_ends_at_win_score
- prediction_reflects_off_walls
- ai_moves_toward_ball
- rng_is_deterministic_and_in_range

## Cross-Compilation
The same source compiles for Windows (`x86_64-pc-windows-gnu`) and macOS unchanged.
Web (WASM) target is also supported by macroquad without code changes.
