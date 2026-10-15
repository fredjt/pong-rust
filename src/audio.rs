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

    // Fallback no-op stubs (section 10.3). The `enabled` guard makes the M mute
    // toggle authoritative at every call site: once real audio synthesis is wired
    // in, nothing should sound while muted, and callers never have to remember to
    // gate each call themselves.
    pub fn play_paddle(&self) { if !self.enabled { return; } }
    pub fn play_wall(&self)   { if !self.enabled { return; } }
    pub fn play_score(&self)  { if !self.enabled { return; } }
    pub fn play_win(&self)    { if !self.enabled { return; } }
    pub fn play_select(&self) { if !self.enabled { return; } }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}
