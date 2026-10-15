use macroquad::prelude::*;

mod game;
mod audio;
mod rnd;

// ─── Section 5.6: Palette (Color constants) ──────────────────────────

const BG: Color = BLACK;
const COURT_LINE: Color = GRAY;
const P1_COLOR: Color = Color::new(0.35, 0.90, 1.00, 1.0);
const P2_COLOR: Color = Color::new(1.00, 0.40, 0.75, 1.0);
const BALL_COLOR: Color = WHITE;
const HUD_DIM: Color = Color::new(1.0, 1.0, 1.0, 0.45);
const DIM_OVERLAY: Color = Color::new(0.0, 0.0, 0.0, 0.62);

// ─── Section 12.2: Window configuration ──────────────────────────────

fn window_conf() -> Conf {
    Conf {
        window_title: "Pong".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,
        ..Default::default()
    }
}

// ─── Helper: center text ─────────────────────────────────────────────

fn center_text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    let dims = measure_text(s, None, size as u16, 1.0);
    draw_text(s, x - dims.width / 2.0, y, size, c);
}

// ─── Section 12.1: Main loop ─────────────────────────────────────────

#[macroquad::main(window_conf)]
async fn main() {
    let mut g = game::new_game();
    let mut audio = audio::Audio::new().await;
    let mut fullscreen = false;

    loop {
        // Auto-close escape hatch for testing (section 16.2)
        if std::env::var("PONG_AUTOCLOSE").is_ok() && get_time() > 1.0 {
            break;
        }

        let dt = get_frame_time().min(game::MAX_FRAME_DT);

        // ── Input reading (section 9) ───────────────────────────────
        let left_up = is_key_down(KeyCode::W);
        let left_down = is_key_down(KeyCode::S);
        let right_up = is_key_down(KeyCode::Up);
        let right_down = is_key_down(KeyCode::Down);

        let mut left_dir: f32 = 0.0;
        if left_down {
            left_dir = -1.0;
        } else if left_up {
            left_dir = 1.0;
        }

        let mut right_dir: f32 = 0.0;
        if g.mode == game::Mode::VsPlayer {
            if right_down {
                right_dir = -1.0;
            } else if right_up {
                right_dir = 1.0;
            }
        }

        // Menu navigation (pressed, not held). The menu has exactly two items,
        // so both directions toggle between index 0 and 1. Using `(x + 1) % 2`
        // for Up as well avoids the `wrapping_sub(1)` underflow that produced
        // usize::MAX (dropped highlight + wrong selection on Enter).
        if is_key_pressed(KeyCode::W) || is_key_pressed(KeyCode::Up) {
            g.menu_index = (g.menu_index + 1) % 2;
            let _ = audio.play_select();
        }
        if is_key_pressed(KeyCode::S) || is_key_pressed(KeyCode::Down) {
            g.menu_index = (g.menu_index + 1) % 2;
            let _ = audio.play_select();
        }

        // Difficulty selection
        if is_key_pressed(KeyCode::Key1) {
            g.difficulty = 0;
            let _ = audio.play_select();
        }
        if is_key_pressed(KeyCode::Key2) {
            g.difficulty = 1;
            let _ = audio.play_select();
        }
        if is_key_pressed(KeyCode::Key3) {
            g.difficulty = 2;
            let _ = audio.play_select();
        }

        // Mode selection is intentionally NOT bound to the 1/2 keys: those are
        // reserved for CPU difficulty above, and binding them here made a single
        // "1" press both set difficulty AND immediately launch a VsPlayer match.
        // The mode is chosen by highlighting an item (Up/Down) then confirming
        // with Enter/Space below.

        // Start match from menu (Enter/Space)
        if (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space))
            && g.phase == game::Phase::Menu
        {
            let mode = if g.menu_index == 0 {
                game::Mode::VsPlayer
            } else {
                game::Mode::VsCpu
            };
            game::start_match(&mut g, mode);
            audio.play_select();
            continue;
        }

        // Pause / resume
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) {
            match g.phase {
                game::Phase::Playing => {
                    g.phase = game::Phase::Paused;
                }
                game::Phase::Paused => {
                    g.phase = game::Phase::Playing;
                }
                _ => {}
            }
        }

        // Quit pause to menu
        if is_key_pressed(KeyCode::Q) && g.phase == game::Phase::Paused {
            g.phase = game::Phase::Menu;
            g.winner = -1;
        }

        // Fullscreen toggle
        if is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }

        // Sound toggle
        if is_key_pressed(KeyCode::M) {
            audio.toggle();
        }

        // Game over: Enter = rematch, Esc = menu
        if g.phase == game::Phase::GameOver {
            if (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space))
                && g.over_timer <= 0.0
            {
                let mode = g.mode;
                game::start_match(&mut g, mode);
                continue;
            }
            if is_key_pressed(KeyCode::Escape) && g.over_timer <= 0.0 {
                g.phase = game::Phase::Menu;
                g.winner = -1;
            }
        }

        // ── Game step ───────────────────────────────────────────────
        let ev = game::step(&mut g, left_dir, right_dir, dt);

        // React to events (audio/effects)
        if let Some(e) = ev {
            match e {
                game::Event::Wall => audio.play_wall(),
                game::Event::PaddleLeft => audio.play_paddle(),
                game::Event::PaddleRight => audio.play_paddle(),
                game::Event::ScoreLeft | game::Event::ScoreRight => audio.play_score(),
                game::Event::MatchOver => audio.play_win(),
            }
        }

        // ── Rendering (section 11) ──────────────────────────────────
        clear_background(BG);

        // Screen shake offsets
        let (shake_ox, shake_oy) = if g.shake > 0.0 {
            let t = g.shake / game::SHAKE_TIME;
            let amp = game::SHAKE_AMPLITUDE * t;
            ((g.ball.x * 10.0).sin() * amp,
             (g.ball.y * 7.0).cos() * amp)
        } else {
            (0.0, 0.0)
        };

        // Layer 2: Court frame
        let wall_thick = 4.0_f32;
        draw_rectangle(0.0, game::WALL_TOP - wall_thick, 1280.0, wall_thick, COURT_LINE);
        draw_rectangle(0.0, game::WALL_BOTTOM, 1280.0, wall_thick, COURT_LINE);
        draw_rectangle(game::LEFT_EDGE - wall_thick, game::WALL_TOP - wall_thick,
                       wall_thick, game::WALL_BOTTOM - game::WALL_TOP + 2.0 * wall_thick, COURT_LINE);
        draw_rectangle(game::RIGHT_EDGE, game::WALL_TOP - wall_thick,
                       wall_thick, game::WALL_BOTTOM - game::WALL_TOP + 2.0 * wall_thick, COURT_LINE);

        // Layer 3: Net (dashed centre line)
        let mut net_y = game::WALL_TOP + 12.0;
        while net_y <= game::WALL_BOTTOM {
            draw_rectangle(game::NET_X - 2.0, net_y, 4.0, 20.0, COURT_LINE);
            net_y += 36.0;
        }

        // Layer 4: Ball trail
        for i in 0..g.trail.count {
            let alpha = 0.35 * (1.0 - i as f32 / game::TRAIL_LEN as f32);
            let size = 2.0 * game::BALL_R * (1.0 - 0.05 * i as f32);
            let (tx, ty) = g.trail.get(i);
            draw_rectangle(tx - size / 2.0 + shake_ox, ty - size / 2.0 + shake_oy,
                           size, size, Color::new(1.0, 1.0, 1.0, alpha));
        }

        // Layer 5: Paddles (with glow)
        for (paddle, is_left) in [(&g.left, true), (&g.right, false)] {
            let color = if paddle.flash > 0.0 { WHITE } else {
                if is_left { P1_COLOR } else { P2_COLOR }
            };
            // Glow
            draw_rectangle(paddle.x - 8.0 + shake_ox, paddle.y - 8.0 + shake_oy,
                           paddle.w + 16.0, paddle.h + 16.0,
                           Color::new(color.r, color.g, color.b, 0.18));
            // Main
            draw_rectangle(paddle.x + shake_ox, paddle.y + shake_oy,
                           paddle.w, paddle.h, color);
        }

        // Layer 6: Ball (skip in Menu)
        if g.phase != game::Phase::Menu {
            let bx = g.ball.x - game::BALL_R + shake_ox;
            let by = g.ball.y - game::BALL_R + shake_oy;
            let size = 2.0 * game::BALL_R;
            // Glow
            draw_rectangle(bx - 3.0, by - 3.0, size + 6.0, size + 6.0,
                           Color::new(1.0, 1.0, 1.0, 0.20));
            // Ball
            draw_rectangle(bx, by, size, size, BALL_COLOR);
        }

        // Layer 7: Scores
        let left_score_str = format!("{}", g.score_left);
        let right_score_str = format!("{}", g.score_right);
        draw_text(&left_score_str, 560.0, 48.0, 96.0, P1_COLOR);
        draw_text(&right_score_str, 688.0, 48.0, 96.0, P2_COLOR);

        // Layer 8: Serve countdown
        if g.phase == game::Phase::Serve {
            let n = (g.serve_timer / game::SERVE_COUNTDOWN * 3.0).ceil() as i32;
            if n >= 1 && n <= 3 {
                center_text(&n.to_string(), game::NET_X, 400.0, 120.0, WHITE);
            }
            center_text("GET READY", game::NET_X, 470.0, 28.0, HUD_DIM);
        }

        // Layer 9: Menu overlay
        if g.phase == game::Phase::Menu {
            draw_rectangle(0.0, 0.0, 1280.0, 720.0, DIM_OVERLAY);
            center_text("PONG // RUST", 640.0, 180.0, 84.0, WHITE);
            draw_text("macroquad edition", 640.0 - measure_text("macroquad edition", None, 22, 1.0).width / 2.0,
                      225.0, 22.0, HUD_DIM);

            let modes = ["VS PLAYER", "VS CPU"];
            for (i, mode_name) in modes.iter().enumerate() {
                let y = 330.0 + i as f32 * 45.0;
                let prefix = if g.menu_index == i { "> " } else { "" };
                let color = if g.menu_index == i { WHITE } else { GRAY };
                draw_text(&format!("{}{}", prefix, mode_name),
                          640.0 - measure_text(mode_name, None, 34, 1.0).width / 2.0 + 8.0,
                          y, 34.0, color);
            }

            let diff_names = ["EASY", "NORMAL", "HARD"];
            let active_diff = diff_names[g.difficulty];
            let diff_text = format!("CPU LEVEL: {} | {} | {}",
                                    if g.difficulty == 0 { active_diff } else { "EASY" },
                                    if g.difficulty == 1 { active_diff } else { "NORMAL" },
                                    if g.difficulty == 2 { active_diff } else { "HARD" });
            // Draw with highlighted word
            let diff_y = 430.0;
            let parts: Vec<&str> = diff_text.split(" | ").collect();
            let mut x_off: f32 = 640.0 - measure_text(&diff_text, None, 26, 1.0).width / 2.0;
            for (&part, _i) in parts.iter().zip(0..) {
                let color = if part == active_diff { YELLOW } else { HUD_DIM };
                draw_text(part, x_off, diff_y, 26.0, color);
                x_off += measure_text(part, None, 26, 1.0).width + 4.0;
            }

            let controls = [
                "P1: W / S      P2: UP / DOWN",
                "ENTER START    ESC PAUSE    M SOUND",
                "F11 FULLSCREEN   UP/DOWN PICK   1-3 CPU",
            ];
            for (i, line) in controls.iter().enumerate() {
                draw_text(line, 640.0 - measure_text(line, None, 22, 1.0).width / 2.0,
                          510.0 + i as f32 * 30.0, 22.0, HUD_DIM);
            }

            draw_text("FIRST TO 7 WINS", 640.0 - measure_text("FIRST TO 7 WINS", None, 20, 1.0).width / 2.0,
                      670.0, 20.0, HUD_DIM);
        }

        // Layer 10: Pause overlay
        if g.phase == game::Phase::Paused {
            draw_rectangle(0.0, 0.0, 1280.0, 720.0, DIM_OVERLAY);
            center_text("PAUSED", 640.0, 340.0, 80.0, WHITE);
            center_text("ESC RESUME   Q MENU", 640.0, 400.0, 26.0, HUD_DIM);
        }

        // Layer 11: Game over overlay
        if g.phase == game::Phase::GameOver {
            draw_rectangle(0.0, 0.0, 1280.0, 720.0, DIM_OVERLAY);
            let winner_text = if g.winner == 0 { "PLAYER 1 WINS" } else { "PLAYER 2 WINS" };
            let win_color = if g.winner == 0 { P1_COLOR } else { P2_COLOR };
            center_text(winner_text, 640.0, 320.0, 72.0, win_color);
            let final_score = format!("{} - {}", g.score_left, g.score_right);
            draw_text(&final_score, 640.0 - measure_text(&final_score, None, 44, 1.0).width / 2.0,
                      390.0, 44.0, WHITE);
            center_text("ENTER REMATCH   ESC MENU", 640.0, 450.0, 26.0, HUD_DIM);
        }

        // Layer 12: Scanlines
        for y in (0..720).step_by(4) {
            draw_rectangle(0.0, y as f32, 1280.0, 1.0, Color::new(0.0, 0.0, 0.0, 0.10));
        }

        // Layer 13: Vignette (simplified: one rect per edge)
        draw_rectangle(0.0, 0.0, 90.0, 720.0, Color::new(0.0, 0.0, 0.0, 0.10));
        draw_rectangle(1190.0, 0.0, 90.0, 720.0, Color::new(0.0, 0.0, 0.0, 0.10));
        draw_rectangle(0.0, 0.0, 1280.0, 90.0, Color::new(0.0, 0.0, 0.0, 0.10));
        draw_rectangle(0.0, 630.0, 1280.0, 90.0, Color::new(0.0, 0.0, 0.0, 0.10));

        // Layer 14: Sound indicator
        let sound_text = if audio.enabled { "SOUND: ON" } else { "SOUND: OFF" };
        draw_text(sound_text, 30.0, 700.0, 18.0, HUD_DIM);

        next_frame().await;
    }
}


