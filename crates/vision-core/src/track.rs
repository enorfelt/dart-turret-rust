use heapless::Vec;

use crate::config::VisionConfig;
use crate::identity::Identity;
use crate::rect::Rect;

pub const MAX_TRACKS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub id: u8,
    pub rect: Rect,
    pub detection: Option<usize>,
    pub identity: Identity,
    pub recognized_ms: Option<u32>,
    misses: u8,
}

#[derive(Debug, Default)]
pub struct Tracker {
    tracks: Vec<Track, MAX_TRACKS>,
    next_id: u8,
}

impl Tracker {
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn tracks_mut(&mut self) -> &mut [Track] {
        &mut self.tracks
    }

    pub fn update(&mut self, detections: &[Rect], config: &VisionConfig) {
        let mut pairs: Vec<(f32, usize, usize), { MAX_TRACKS * MAX_TRACKS }> = Vec::new();
        for (t, track) in self.tracks.iter().enumerate() {
            for (d, rect) in detections.iter().enumerate().take(MAX_TRACKS) {
                let iou = track.rect.iou(rect);
                if iou >= config.track_iou {
                    let _ = pairs.push((iou, t, d));
                }
            }
        }
        pairs.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));

        for track in self.tracks.iter_mut() {
            track.detection = None;
        }
        let mut claimed = [false; MAX_TRACKS];
        for (_, t, d) in pairs {
            if claimed[d] || self.tracks[t].detection.is_some() {
                continue;
            }
            claimed[d] = true;
            self.tracks[t].detection = Some(d);
            self.tracks[t].rect = detections[d];
            self.tracks[t].misses = 0;
        }

        self.tracks.retain_mut(|track| {
            if track.detection.is_none() {
                track.misses = track.misses.saturating_add(1);
            }
            track.misses <= config.track_max_misses
        });

        for (d, rect) in detections.iter().enumerate().take(MAX_TRACKS) {
            if claimed[d] {
                continue;
            }
            let id = self.fresh_id();
            let track = Track {
                id,
                rect: *rect,
                detection: Some(d),
                identity: Identity::default(),
                recognized_ms: None,
                misses: 0,
            };
            if self.tracks.push(track).is_err() {
                break;
            }
        }
    }

    fn fresh_id(&mut self) -> u8 {
        loop {
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);
            if self.tracks.iter().all(|track| track.id != id) {
                return id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: VisionConfig = VisionConfig {
        min_face_score: 0.5,
        track_iou: 0.3,
        track_max_misses: 2,
        match_similarity: 0.5,
        match_margin: 0.1,
        target_confirmations: 3,
        recheck_ms: 1_000,
        enroll_samples: 3,
        enroll_timeout_ms: 10_000,
    };

    fn ids(tracker: &Tracker) -> alloc::vec::Vec<(u8, Option<usize>)> {
        tracker
            .tracks()
            .iter()
            .map(|track| (track.id, track.detection))
            .collect()
    }

    #[test]
    fn moving_face_keeps_its_track() {
        let mut tracker = Tracker::default();
        tracker.update(&[Rect::new(0, 0, 40, 40)], &CONFIG);
        tracker.update(&[Rect::new(5, 0, 45, 40)], &CONFIG);

        assert_eq!(ids(&tracker), [(0, Some(0))]);
        assert_eq!(tracker.tracks()[0].rect, Rect::new(5, 0, 45, 40));
    }

    #[test]
    fn detections_are_matched_by_overlap_not_order() {
        let mut tracker = Tracker::default();
        tracker.update(
            &[Rect::new(0, 0, 40, 40), Rect::new(100, 0, 140, 40)],
            &CONFIG,
        );
        tracker.update(
            &[Rect::new(102, 0, 142, 40), Rect::new(2, 0, 42, 40)],
            &CONFIG,
        );

        assert_eq!(ids(&tracker), [(0, Some(1)), (1, Some(0))]);
    }

    #[test]
    fn jump_starts_a_new_track() {
        let mut tracker = Tracker::default();
        tracker.update(&[Rect::new(0, 0, 40, 40)], &CONFIG);
        tracker.update(&[Rect::new(200, 0, 240, 40)], &CONFIG);

        assert_eq!(ids(&tracker), [(0, None), (1, Some(0))]);
    }

    #[test]
    fn missed_tracks_survive_a_few_frames_then_drop() {
        let mut tracker = Tracker::default();
        let face = Rect::new(0, 0, 40, 40);
        tracker.update(&[face], &CONFIG);
        tracker.update(&[], &CONFIG);
        tracker.update(&[], &CONFIG);
        assert_eq!(ids(&tracker), [(0, None)]);

        tracker.update(&[face], &CONFIG);
        assert_eq!(ids(&tracker), [(0, Some(0))]);

        for _ in 0..3 {
            tracker.update(&[], &CONFIG);
        }
        assert!(tracker.tracks().is_empty());
    }

    #[test]
    fn new_track_after_drop_gets_a_new_id() {
        let mut tracker = Tracker::default();
        tracker.update(&[Rect::new(0, 0, 40, 40)], &CONFIG);
        for _ in 0..3 {
            tracker.update(&[], &CONFIG);
        }
        tracker.update(&[Rect::new(0, 0, 40, 40)], &CONFIG);

        assert_eq!(ids(&tracker), [(1, Some(0))]);
    }

    #[test]
    fn tracks_are_capped() {
        let mut tracker = Tracker::default();
        let faces: alloc::vec::Vec<Rect> = (0..12)
            .map(|i| Rect::new(i * 50, 0, i * 50 + 40, 40))
            .collect();

        tracker.update(&faces, &CONFIG);

        assert_eq!(tracker.tracks().len(), MAX_TRACKS);
    }
}
