#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisionConfig {
    pub min_face_score: f32,
    pub track_iou: f32,
    pub track_max_misses: u8,
    pub match_similarity: f32,
    pub match_margin: f32,
    pub target_confirmations: u8,
    pub recheck_ms: u32,
    pub enroll_samples: u8,
    pub enroll_timeout_ms: u32,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            min_face_score: 0.5,
            track_iou: 0.3,
            track_max_misses: 5,
            match_similarity: 0.5,
            match_margin: 0.1,
            target_confirmations: 3,
            recheck_ms: 1_000,
            enroll_samples: 3,
            enroll_timeout_ms: 10_000,
        }
    }
}
