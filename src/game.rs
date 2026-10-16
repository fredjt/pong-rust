use crate::rnd;

// ─── Section 5: Constants ────────────────────────────────────────────

pub const WALL_TOP: f32 = 64.0;
pub const WALL_BOTTOM: f32 = 704.0;
pub const LEFT_EDGE: f32 = 24.0;
pub const RIGHT_EDGE: f32 = 1256.0;
pub const NET_X: f32 = 640.0;
pub const PADDLE_W: f32 = 16.0;
pub const PADDLE_H: f32 = 96.0;
pub const LEFT_PADDLE_X: f32 = 80.0;
pub const RIGHT_PADDLE_X: f32 = 1184.0;
pub const PADDLE_MIN_Y: f32 = 64.0;
pub const PADDLE_MAX_Y_BOTTOM: f32 = 704.0;
pub const BALL_R: f32 = 9.0;

pub const PADDLE_SPEED: f32 = 620.0;
pub const SERVE_SPEED: f32 = 430.0;
pub const MAX_BALL_SPEED: f32 = 980.0;
pub const SPEED_UP: f32 = 1.045;
pub const MAX_BOUNCE_DEG: f32 = 50.0;
pub const SUBSTEP_DT: f32 = 0.004;
pub const MAX_FRAME_DT: f32 = 0.05;
pub const PADDLE_COOLDOWN: f32 = 0.10;

pub const WIN_SCORE: i32 = 7;
pub const SERVE_COUNTDOWN: f32 = 1.6;
pub const GAMEOVER_HOLD: f32 = 0.45;

pub const AI_SPEED_EASY: f32 = 380.0;
pub const AI_SPEED_NORMAL: f32 = 520.0;
pub const AI_SPEED_HARD: f32 = 700.0;
pub const AI_ERROR_EASY: f32 = 64.0;
pub const AI_ERROR_NORMAL: f32 = 30.0;
pub const AI_ERROR_HARD: f32 = 10.0;
// Minimum vertical distance (px) the AI will ignore before correcting its aim.
pub const AI_DEADZONE: f32 = 6.0;

pub const TRAIL_LEN: usize = 10;
pub const SHAKE_AMPLITUDE: f32 = 9.0;
pub const SHAKE_TIME: f32 = 0.32;
pub const FLASH_TIME: f32 = 0.10;

// ─── Section 6: Data Model ───────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Wall,
    PaddleLeft,
    PaddleRight,
    ScoreLeft,
    ScoreRight,
    MatchOver,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    VsPlayer,
    VsCpu,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Menu,
    Serve,
    Playing,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Paddle {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub flash: f32,
    pub cooldown: f32,
}

#[derive(Clone, Copy)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub last_hit_by: i32,
}

#[derive(Clone, Copy)]
pub struct Trail {
    pub xs: [f32; TRAIL_LEN],
    pub ys: [f32; TRAIL_LEN],
    pub head: usize,
    pub count: usize,
}

impl Trail {
    pub fn new() -> Self {
        Trail {
            xs: [0.0; TRAIL_LEN],
            ys: [0.0; TRAIL_LEN],
            head: 0,
            count: 0,
        }
    }

    pub fn push(&mut self, x: f32, y: f32) {
        self.xs[self.head] = x;
        self.ys[self.head] = y;
        self.head = (self.head + 1) % TRAIL_LEN;
        if self.count < TRAIL_LEN {
            self.count += 1;
        }
    }

    pub fn get(&self, i: usize) -> (f32, f32) {
        if i >= self.count {
            return (0.0, 0.0);
        }
        let idx = if self.head >= self.count {
            (self.head + TRAIL_LEN - 1 - i) % TRAIL_LEN
        } else {
            if self.head == 0 {
                (TRAIL_LEN - 1 - i) % TRAIL_LEN
            } else {
                (self.head - 1 + TRAIL_LEN - i) % TRAIL_LEN
            }
        };
        (self.xs[idx], self.ys[idx])
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.head = 0;
    }
}

#[derive(Clone, Copy)]
pub struct Game {
    pub left: Paddle,
    pub right: Paddle,
    pub ball: Ball,
    pub trail: Trail,
    pub score_left: i32,
    pub score_right: i32,
    pub mode: Mode,
    pub phase: Phase,
    pub difficulty: usize,
    pub serve_timer: f32,
    pub serve_dir: f32,
    pub over_timer: f32,
    pub shake: f32,
    pub rng: u64,
    pub winner: i32,
    pub menu_index: usize,
    pub ai_aim: f32,
}

// ─── Section 6.2: Required Functions ─────────────────────────────────

pub fn new_game() -> Game {
    let cy = (WALL_TOP + WALL_BOTTOM) * 0.5 - PADDLE_H * 0.5;
    Game {
        left: Paddle {
            x: LEFT_PADDLE_X,
            y: cy,
            w: PADDLE_W,
            h: PADDLE_H,
            flash: 0.0,
            cooldown: 0.0,
        },
        right: Paddle {
            x: RIGHT_PADDLE_X,
            y: cy,
            w: PADDLE_W,
            h: PADDLE_H,
            flash: 0.0,
            cooldown: 0.0,
        },
        ball: Ball {
            x: NET_X,
            y: (WALL_TOP + WALL_BOTTOM) * 0.5,
            vx: 0.0,
            vy: 0.0,
            last_hit_by: -1,
        },
        trail: Trail::new(),
        score_left: 0,
        score_right: 0,
        mode: Mode::VsPlayer,
        phase: Phase::Menu,
        difficulty: 1,
        serve_timer: SERVE_COUNTDOWN,
        serve_dir: 1.0,
        over_timer: 0.0,
        shake: 0.0,
        rng: 0x9E3779B97F4A7C15,
        winner: -1,
        menu_index: 0,
        ai_aim: 0.0,
    }
}

pub fn start_match(g: &mut Game, mode: Mode) {
    // Derive the opening serve direction from the *previous* match's result so
    // serves stay consistent across rematches instead of always flying right.
    // This mirrors the in-match rule in `substep`: the ball is served toward the
    // player who lost the previous point (i.e. away from its winner).
    //   g.winner == 0  -> left won last time -> serve toward right (+1)
    //   g.winner == 1  -> right won last time -> serve toward left  (-1)
    //   g.winner == -1 -> no prior result (fresh game / returned to menu): default right.
    // NOTE: read `g.winner` before it is reset to -1 a few lines below.
    g.serve_dir = match g.winner {
        0 => 1.0,
        1 => -1.0,
        _ => 1.0,
    };
    g.score_left = 0;
    g.score_right = 0;
    g.mode = mode;
    g.phase = Phase::Serve;
    g.serve_timer = SERVE_COUNTDOWN;
    g.winner = -1;
    g.over_timer = 0.0;
    g.shake = 0.0;
    let cy = (WALL_TOP + WALL_BOTTOM) * 0.5 - PADDLE_H * 0.5;
    g.left.y = cy;
    g.right.y = cy;
    g.ball.x = NET_X;
    g.ball.y = (WALL_TOP + WALL_BOTTOM) * 0.5;
    g.ball.vx = 0.0;
    g.ball.vy = 0.0;
    g.ball.last_hit_by = -1;
    g.trail.clear();
}

pub fn move_paddle(g: &mut Game, side: i32, dir: f32, dt: f32) {
    let speed = if side == 0 {
        PADDLE_SPEED
    } else {
        // Use AI speed for right paddle when in VsCpu mode — but actually
        // the caller handles AI separately; here we just use PADDLE_SPEED.
        PADDLE_SPEED
    };
    let mut paddle = if side == 0 { g.left } else { g.right };
    paddle.y += dir * speed * dt;
    let max_y = PADDLE_MAX_Y_BOTTOM - PADDLE_H;
    paddle.y = paddle.y.clamp(PADDLE_MIN_Y, max_y);
    if side == 0 {
        g.left = paddle;
    } else {
        g.right = paddle;
    }
}

pub fn serve_ball(g: &mut Game) {
    g.ball.x = NET_X;
    g.ball.y = (WALL_TOP + WALL_BOTTOM) * 0.5;
    let angle = rnd::range(&mut g.rng, -20.0_f32.to_radians(), 20.0_f32.to_radians());
    let dir = g.serve_dir;
    g.ball.vx = dir * SERVE_SPEED * angle.cos();
    g.ball.vy = SERVE_SPEED * angle.sin();
    g.ball.last_hit_by = -1;
    // Set ai_aim for the next point
    let d = g.difficulty;
    let err = match d {
        0 => AI_ERROR_EASY,
        1 => AI_ERROR_NORMAL,
        _ => AI_ERROR_HARD,
    };
    g.ai_aim = rnd::range(&mut g.rng, -err, err);
}

pub fn ball_aabb_hits_paddle(g: &Game, p: &Paddle) -> bool {
    let bx1 = g.ball.x - BALL_R;
    let by1 = g.ball.y - BALL_R;
    let bx2 = g.ball.x + BALL_R;
    let by2 = g.ball.y + BALL_R;
    let px = p.x;
    let py = p.y;
    let pw = p.w;
    let ph = p.h;
    bx1 < px + pw && bx2 > px && by1 < py + ph && by2 > py
}

pub fn predict_y(g: &Game, target_x: f32) -> f32 {
    if g.ball.vx.abs() < 0.0001 {
        return g.ball.y;
    }
    let t = (target_x - g.ball.x) / g.ball.vx;
    if t < 0.0 {
        return g.ball.y;
    }
    let mut y = g.ball.y + g.ball.vy * t;
    let top = WALL_TOP + BALL_R;
    let bot = WALL_BOTTOM - BALL_R;
    // Mirror-unfold: max 8 iterations
    for _ in 0..8 {
        while y < top {
            y = top + (top - y);
        }
        while y > bot {
            y = bot - (y - bot);
        }
    }
    y.clamp(top, bot)
}

/// Tick a timer: subtract dt, floor at 0.0
fn tick(t: &mut f32, dt: f32) {
    *t -= dt;
    if *t < 0.0 {
        *t = 0.0;
    }
}

pub fn ai_intent(g: &Game, _dt: f32) -> f32 {
    let d = g.difficulty;
    let _ai_spd: f32 = match d {
        0 => AI_SPEED_EASY,
        1 => AI_SPEED_NORMAL,
        _ => AI_SPEED_HARD,
    };
    let _ai_err: f32 = match d {
        0 => AI_ERROR_EASY,
        1 => AI_ERROR_NORMAL,
        _ => AI_ERROR_HARD,
    };

    // Only compute if ball is approaching the AI
    let target_y = if g.ball.vx > 0.0 {
        let predicted = predict_y(g, RIGHT_PADDLE_X - BALL_R);
        predicted + g.ai_aim
    } else {
        (WALL_TOP + WALL_BOTTOM) * 0.5
    };

    let want = target_y - PADDLE_H * 0.5;
    let want = want.clamp(PADDLE_MIN_Y, PADDLE_MAX_Y_BOTTOM - PADDLE_H);
    let delta = want - g.right.y;

    if delta.abs() < AI_DEADZONE {
        return 0.0;
    }
    delta.signum()
}

pub fn step(g: &mut Game, left_dir: f32, right_dir: f32, dt: f32) -> Option<Event> {
    let dt = dt.min(MAX_FRAME_DT);

    // 1. Decay timers
    tick(&mut g.shake, dt);
    tick(&mut g.left.flash, dt);
    tick(&mut g.right.flash, dt);
    tick(&mut g.left.cooldown, dt);
    tick(&mut g.right.cooldown, dt);
    tick(&mut g.over_timer, dt);

    // 2. Move paddles
    move_paddle(g, 0, left_dir, dt);
    if g.mode == Mode::VsCpu {
        let ai_d = ai_intent(g, dt);
        let d = g.difficulty;
        let ai_spd: f32 = match d {
            0 => AI_SPEED_EASY,
            1 => AI_SPEED_NORMAL,
            _ => AI_SPEED_HARD,
        };
        // Move right paddle with AI speed
        let mut paddle = g.right;
        paddle.y += ai_d * ai_spd * dt;
        let max_y = PADDLE_MAX_Y_BOTTOM - PADDLE_H;
        paddle.y = paddle.y.clamp(PADDLE_MIN_Y, max_y);
        g.right = paddle;
    } else {
        move_paddle(g, 1, right_dir, dt);
    }

    // 3. Serve phase
    if g.phase == Phase::Serve {
        g.serve_timer -= dt;
        if g.serve_timer <= 0.0 {
            serve_ball(g);
            g.phase = Phase::Playing;
        }
        return None;
    }

    // If not playing, nothing happens
    if g.phase != Phase::Playing {
        return None;
    }

    // 4. Sub-stepping — advance exactly `dt` total by splitting it into n equal
    // steps of `h`. Using h (instead of the fixed SUBSTEP_DT) keeps simulated
    // time locked to real time at any frame rate, so the ball never drifts
    // faster than intended on non-standard refresh rates. `.max(1)` guards
    // against a zero-length frame yielding n == 0 and a divide-by-zero `h`.
    let n = ((dt / SUBSTEP_DT).ceil() as i32).max(1);
    let h = dt / n as f32;
    let mut events: Vec<Event> = Vec::new();

    for _ in 0..n {
        substep(g, h, &mut events);
    }

    // 5. Push trail once per frame
    g.trail.push(g.ball.x, g.ball.y);

    // Return the last event (for backward compatibility with caller)
    events.last().copied()
}

fn substep(g: &mut Game, dt: f32, events: &mut Vec<Event>) {
    // Move ball
    g.ball.x += g.ball.vx * dt;
    g.ball.y += g.ball.vy * dt;

    // Top wall
    if g.ball.y - BALL_R < WALL_TOP && g.ball.vy < 0.0 {
        g.ball.y = WALL_TOP + BALL_R;
        g.ball.vy = -g.ball.vy;
        events.push(Event::Wall);
    }

    // Bottom wall
    if g.ball.y + BALL_R > WALL_BOTTOM && g.ball.vy > 0.0 {
        g.ball.y = WALL_BOTTOM - BALL_R;
        g.ball.vy = -g.ball.vy;
        events.push(Event::Wall);
    }

    // Left paddle
    if g.ball.vx < 0.0 && g.left.cooldown <= 0.0 && ball_aabb_hits_paddle(g, &g.left) {
        let lx = g.left.x;
        let ly = g.left.y;
        let lw = g.left.w;
        let lh = g.left.h;
        reflect_off_paddle(g, lx, ly, lw, lh, 1.0);
        g.left.cooldown = PADDLE_COOLDOWN;
        g.left.flash = FLASH_TIME;
        g.ball.last_hit_by = 0;
        events.push(Event::PaddleLeft);
    }

    // Right paddle
    if g.ball.vx > 0.0 && g.right.cooldown <= 0.0 && ball_aabb_hits_paddle(g, &g.right) {
        let rx = g.right.x;
        let ry = g.right.y;
        let rw = g.right.w;
        let rh = g.right.h;
        reflect_off_paddle(g, rx, ry, rw, rh, -1.0);
        g.right.cooldown = PADDLE_COOLDOWN;
        g.right.flash = FLASH_TIME;
        g.ball.last_hit_by = 1;
        events.push(Event::PaddleRight);
    }

    // Scoring
    if g.ball.x + BALL_R < LEFT_EDGE - 40.0 {
        // Right player scores
        g.score_right += 1;
        g.shake = SHAKE_TIME;
        g.trail.clear();
        g.ball.last_hit_by = -1;
        g.ball.x = NET_X;
        g.ball.y = (WALL_TOP + WALL_BOTTOM) * 0.5;
        g.ball.vx = 0.0;
        g.ball.vy = 0.0;

        if g.score_right >= WIN_SCORE {
            g.winner = 1;
            g.phase = Phase::GameOver;
            g.over_timer = GAMEOVER_HOLD;
            events.push(Event::MatchOver);
        } else {
            g.phase = Phase::Serve;
            g.serve_timer = SERVE_COUNTDOWN;
            g.serve_dir = -1.0; // serve toward loser (left)
            events.push(Event::ScoreRight);
        }
    }

    if g.ball.x - BALL_R > RIGHT_EDGE + 40.0 {
        // Left player scores
        g.score_left += 1;
        g.shake = SHAKE_TIME;
        g.trail.clear();
        g.ball.last_hit_by = -1;
        g.ball.x = NET_X;
        g.ball.y = (WALL_TOP + WALL_BOTTOM) * 0.5;
        g.ball.vx = 0.0;
        g.ball.vy = 0.0;

        if g.score_left >= WIN_SCORE {
            g.winner = 0;
            g.phase = Phase::GameOver;
            g.over_timer = GAMEOVER_HOLD;
            events.push(Event::MatchOver);
        } else {
            g.phase = Phase::Serve;
            g.serve_timer = SERVE_COUNTDOWN;
            g.serve_dir = 1.0; // serve toward loser (right)
            events.push(Event::ScoreLeft);
        }
    }
}

fn reflect_off_paddle(g: &mut Game, px: f32, py: f32, pw: f32, ph: f32, dirx: f32) {
    let rel = ((g.ball.y - (py + ph * 0.5)) / (ph * 0.5)).clamp(-1.0, 1.0);
    let max_rad = MAX_BOUNCE_DEG.to_radians();
    let angle = rel * max_rad;
    let speed = (g.ball.vx * g.ball.vx + g.ball.vy * g.ball.vy).sqrt().max(SERVE_SPEED) * SPEED_UP;
    let speed = speed.min(MAX_BALL_SPEED);

    g.ball.vx = dirx * speed * angle.cos();
    g.ball.vy = angle.sin() * speed;

    // Separate bodies
    if dirx > 0.0 {
        g.ball.x = px + pw + BALL_R + 1.0;
    } else {
        g.ball.x = px - BALL_R - 1.0;
    }
}

// ─── Section 15: Unit Tests ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rnd;

    #[test]
    fn ball_bounces_off_top_wall() {
        let mut g = new_game();
        g.phase = Phase::Playing;
        g.ball.x = 640.0;
        g.ball.y = WALL_TOP + BALL_R + 1.0;
        g.ball.vy = -500.0;
        g.ball.vx = 0.0;
        let _ = step(&mut g, 0.0, 0.0, 1.0 / 60.0);
        assert!(g.ball.vy > 0.0, "vy should be positive after bouncing off top wall");
        assert!(g.ball.y >= WALL_TOP + BALL_R - 0.001, "ball y should be at least WALL_TOP + BALL_R");
    }

    #[test]
    fn ball_bounces_off_bottom_wall() {
        let mut g = new_game();
        g.phase = Phase::Playing;
        g.ball.x = 640.0;
        g.ball.y = WALL_BOTTOM - BALL_R - 1.0;
        g.ball.vy = 500.0;
        g.ball.vx = 0.0;
        let _ = step(&mut g, 0.0, 0.0, 1.0 / 60.0);
        assert!(g.ball.vy < 0.0, "vy should be negative after bouncing off bottom wall");
        assert!(g.ball.y <= WALL_BOTTOM - BALL_R + 0.001, "ball y should be at most WALL_BOTTOM - BALL_R");
    }

    #[test]
    fn paddle_hit_flips_direction_and_speeds_up() {
        let mut g = new_game();
        g.phase = Phase::Playing;
        let centre_y = (WALL_TOP + WALL_BOTTOM) * 0.5;
        g.ball.x = LEFT_PADDLE_X + PADDLE_W + BALL_R;
        g.ball.y = centre_y;
        g.ball.vx = -430.0;
        g.ball.vy = 0.0;
        let _ = step(&mut g, 0.0, 0.0, SUBSTEP_DT);
        assert!(g.ball.vx > 0.0, "vx should be positive after hitting left paddle");
        let speed = (g.ball.vx * g.ball.vx + g.ball.vy * g.ball.vy).sqrt();
        assert!(speed >= 430.0 * SPEED_UP - 0.5, "speed should increase: {} >= {}", speed, 430.0 * SPEED_UP);
    }

    #[test]
    fn edge_hit_produces_steep_angle() {
        let mut g = new_game();
        g.phase = Phase::Playing;
        let cy = (WALL_TOP + WALL_BOTTOM) * 0.5 - PADDLE_H * 0.5;
        g.left.y = cy;
        // Position ball at the bottom edge of the paddle
        let paddle_bottom_edge = cy + PADDLE_H;
        g.ball.x = LEFT_PADDLE_X + PADDLE_W + BALL_R;
        g.ball.y = paddle_bottom_edge - BALL_R * 0.2;
        g.ball.vx = -430.0;
        g.ball.vy = 0.0;
        let _ = step(&mut g, 0.0, 0.0, SUBSTEP_DT);
        assert!(g.ball.vy.abs() > g.ball.vx.abs() * 0.7, "edge hit should produce steep angle: vy={}, vx={}", g.ball.vy, g.ball.vx);
    }

    #[test]
    fn paddle_is_clamped_to_court() {
        let mut g = new_game();
        move_paddle(&mut g, 0, -1.0, 5.0);
        assert!(g.left.y >= PADDLE_MIN_Y - 0.001, "left paddle should be clamped to minimum y");
        move_paddle(&mut g, 0, 1.0, 20.0);
        assert!(g.left.y <= PADDLE_MAX_Y_BOTTOM - PADDLE_H + 0.001, "left paddle should be clamped to maximum y");
    }

    #[test]
    fn ball_cannot_tunnel_through_paddle() {
        let mut g = new_game();
        g.phase = Phase::Playing;
        let centre_y = (WALL_TOP + WALL_BOTTOM) * 0.5 - PADDLE_H * 0.5;
        g.left.y = centre_y;
        g.ball.x = LEFT_PADDLE_X + PADDLE_W + BALL_R + 4.0;
        g.ball.y = centre_y;
        g.ball.vx = -2000.0;
        g.ball.vy = 0.0;
        let _ = step(&mut g, 0.0, 0.0, 1.0 / 60.0);
        assert!(g.ball.vx > 0.0, "ball should bounce off paddle even at high speed");
    }

    #[test]
    fn scoring_awards_point_and_serves_against_loser() {
        let mut g = new_game();
        g.phase = Phase::Playing;
        // Start the ball already past the left goal line so a single frame's
        // motion registers the point regardless of sub-step size. The old
        // `LEFT_EDGE - 41` value only scored because step() over-simulated ~20%
        // extra time per 60fps frame; that bug is now fixed, so the test must
        // position the ball to cross within a real frame.
        g.ball.x = LEFT_EDGE - 50.0;
        g.ball.y = 384.0;
        g.ball.vx = -400.0;
        let _ = step(&mut g, 0.0, 0.0, 1.0 / 60.0);
        assert_eq!(g.score_right, 1, "right player should score");
        assert_eq!(g.phase, Phase::Serve, "should transition to serve");
        assert_eq!(g.serve_dir, -1.0, "serve direction should be toward loser (left)");
    }

    #[test]
    fn match_ends_at_win_score() {
        let mut g = new_game();
        g.phase = Phase::Playing;
        g.score_right = WIN_SCORE - 1;
        // See scoring test: start past the goal line so the final point lands
        // within one real frame now that step() no longer over-simulates.
        g.ball.x = LEFT_EDGE - 50.0;
        g.ball.y = 384.0;
        g.ball.vx = -400.0;
        let _ = step(&mut g, 0.0, 0.0, 1.0 / 60.0);
        assert_eq!(g.phase, Phase::GameOver, "should be game over");
        assert_eq!(g.winner, 1, "right player should win");
    }

    #[test]
    fn prediction_reflects_off_walls() {
        let mut g = new_game();
        g.ball.x = 640.0;
        g.ball.y = WALL_TOP + BALL_R + 5.0;
        g.ball.vy = -900.0;
        g.ball.vx = 300.0;
        let result = predict_y(&g, RIGHT_PADDLE_X);
        assert!(result >= WALL_TOP + BALL_R, "predicted y should be within walls");
        assert!(result <= WALL_BOTTOM - BALL_R, "predicted y should be within walls");
    }

    #[test]
    fn ai_moves_toward_ball() {
        let mut g = new_game();
        g.mode = Mode::VsCpu;
        g.ball.x = 800.0;
        g.ball.y = 200.0;
        g.ball.vx = 300.0; // approaching right paddle
        g.right.y = 600.0;
        let intent = ai_intent(&g, 1.0 / 60.0);
        assert!(intent < 0.0, "AI should move up (negative direction) toward ball");
    }

    #[test]
    fn rng_is_deterministic_and_in_range() {
        let mut s1: u64 = 12345;
        let mut s2: u64 = 12345;
        let mut outputs1 = Vec::new();
        let mut outputs2 = Vec::new();
        for _ in 0..5 {
            outputs1.push(rnd::next_f32(&mut s1));
            outputs2.push(rnd::next_f32(&mut s2));
        }
        assert_eq!(outputs1, outputs2, "same seed should produce same outputs");
        for val in &outputs1 {
            assert!(*val >= 0.0 && *val <= 1.0, "rng output should be in [0, 1]");
        }
    }

    #[test]
    fn start_match_serves_toward_previous_loser() {
        // Left won the previous match -> serve toward the right (the loser).
        let mut g = new_game();
        g.winner = 0;
        start_match(&mut g, Mode::VsPlayer);
        assert_eq!(g.serve_dir, 1.0, "after left wins, next serve goes toward right");

        // Right won the previous match -> serve toward the left (the loser).
        let mut g = new_game();
        g.winner = 1;
        start_match(&mut g, Mode::VsPlayer);
        assert_eq!(g.serve_dir, -1.0, "after right wins, next serve goes toward left");

        // No previous result -> default to serving right.
        let mut g = new_game();
        g.winner = -1;
        start_match(&mut g, Mode::VsPlayer);
        assert_eq!(g.serve_dir, 1.0, "fresh match defaults to serving right");
    }

    #[test]
    fn step_advances_exactly_dt() {
        // A free-flying ball must move exactly vx*dt / vy*dt over one frame:
        // sub-stepping splits dt evenly and no longer over-simulates real time.
        let mut g = new_game();
        g.phase = Phase::Playing;
        g.ball.x = 600.0;
        g.ball.y = 384.0;
        g.ball.vx = 500.0;
        g.ball.vy = 120.0;
        let dt = 1.0 / 60.0; // deliberately NOT a multiple of SUBSTEP_DT
        let x0 = g.ball.x;
        let y0 = g.ball.y;
        let _ = step(&mut g, 0.0, 0.0, dt);
        assert!(
            (g.ball.x - (x0 + 500.0 * dt)).abs() < 1e-3,
            "x should advance by exactly vx*dt: got {}",
            g.ball.x
        );
        assert!(
            (g.ball.y - (y0 + 120.0 * dt)).abs() < 1e-3,
            "y should advance by exactly vy*dt: got {}",
            g.ball.y
        );
    }
}
