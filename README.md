# PONG // RUST — A CRT-styled Pong game in Rust + Macroquad

A single-window, local-multiplayer-and-solo Pong game. Compiles to one small native binary for Windows, macOS and Linux with no runtime dependencies and no asset files.

## Build

```sh
cargo build --release
```

Run: `./target/release/pong-rust`

## Controls

| Action | Keys |
|---|---|
| Player 1 (left paddle) | W / S |
| Player 2 (right paddle) | Up / Down arrows |
| Start / Rematch | Enter or Space |
| Pause / Resume | Esc or P |
| Quit to menu (from pause) | Q |
| Toggle sound | M |
| Fullscreen toggle | F11 |
| Select mode (menu) | 1 = VS Player, 2 = VS CPU |
| CPU difficulty (menu) | 1 = Easy, 2 = Normal, 3 = Hard |

First to 7 points wins.
