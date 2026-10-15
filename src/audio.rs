/// Audio subsystem — fallback mode (section 10.3)
/// When the "audio" feature is disabled or the device is unavailable,
/// all play_* methods are no-ops and Audio::new() returns a muted struct.

pub struct Audio {
    pub enabled: bool,
}

impl Audio {
    /// Create audio subsystem. In fallback mode this is instant.
    pub async fn new() -> Self {
        Audio { enabled: false }
    }

    pub fn play_paddle(&self) {}
    pub fn play_wall(&self) {}
    pub fn play_score(&self) {}
    pub fn play_win(&self) {}
    pub fn play_select(&self) {}

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}
