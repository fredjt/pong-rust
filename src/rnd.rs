/// Deterministic PRNG helper — xorshift64*
pub fn next_f32(state: &mut u64) -> f32 {
    let mut x = *state;
    x ^= x.wrapping_shl(13);
    x ^= x.wrapping_shr(7);
    x ^= x.wrapping_shl(17);
    *state = x;
    ((x.wrapping_mul(0x2545F4914F6CDD1D)) >> 11) as f32 / 9007199254740992.0
}

pub fn range(state: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * next_f32(state)
}
