use core::cmp::Reverse;

use crate::config::VisionConfig;
use crate::identity::Class;
use crate::track::Track;

pub fn next_to_recognize(tracks: &[Track], now_ms: u32, config: &VisionConfig) -> Option<usize> {
    tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| track.detection.is_some() && !track.identity.is_settled())
        .filter(
            |(_, track)| match (track.identity.class(), track.recognized_ms) {
                (_, None) | (Class::Unknown, _) => true,
                (_, Some(at)) => now_ms.wrapping_sub(at) >= config.recheck_ms,
            },
        )
        .min_by_key(|(_, track)| match track.recognized_ms {
            None => (0, Reverse(0)),
            Some(at) => (1, Reverse(now_ms.wrapping_sub(at))),
        })
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gallery::{Candidate, Matches};
    use crate::people::Role;
    use crate::rect::Rect;
    use crate::track::Tracker;

    const CONFIG: VisionConfig = VisionConfig {
        min_face_score: 0.5,
        track_iou: 0.3,
        track_max_misses: 2,
        match_similarity: 0.5,
        match_margin: 0.1,
        target_confirmations: 1,
        recheck_ms: 1_000,
        enroll_samples: 3,
        enroll_timeout_ms: 10_000,
    };

    fn tracker(faces: usize) -> Tracker {
        let mut tracker = Tracker::default();
        let rects: alloc::vec::Vec<Rect> = (0..faces as i16)
            .map(|i| Rect::new(i * 50, 0, i * 50 + 40, 40))
            .collect();
        tracker.update(&rects, &CONFIG);
        tracker
    }

    fn matched(role: Role) -> Matches {
        Matches {
            best: Some(Candidate {
                person: 1,
                role,
                similarity: 0.9,
            }),
            runner_up: None,
        }
    }

    #[test]
    fn never_recognized_tracks_go_first() {
        let mut tracker = tracker(2);
        tracker.tracks_mut()[0].recognized_ms = Some(100);

        assert_eq!(next_to_recognize(tracker.tracks(), 200, &CONFIG), Some(1));
    }

    #[test]
    fn unknown_tracks_round_robin_by_age() {
        let mut tracker = tracker(3);
        tracker.tracks_mut()[0].recognized_ms = Some(300);
        tracker.tracks_mut()[1].recognized_ms = Some(100);
        tracker.tracks_mut()[2].recognized_ms = Some(200);

        assert_eq!(next_to_recognize(tracker.tracks(), 400, &CONFIG), Some(1));
    }

    #[test]
    fn confirmed_targets_wait_for_the_recheck_interval() {
        let mut tracker = tracker(1);
        let track = &mut tracker.tracks_mut()[0];
        track.identity.observe(&matched(Role::Target), &CONFIG);
        track.recognized_ms = Some(1_000);

        assert_eq!(next_to_recognize(tracker.tracks(), 1_999, &CONFIG), None);
        assert_eq!(next_to_recognize(tracker.tracks(), 2_000, &CONFIG), Some(0));
    }

    #[test]
    fn protected_and_missed_tracks_are_skipped() {
        let mut tracker = tracker(2);
        tracker.tracks_mut()[0]
            .identity
            .observe(&matched(Role::Protected), &CONFIG);
        tracker.tracks_mut()[1].detection = None;

        assert_eq!(next_to_recognize(tracker.tracks(), 0, &CONFIG), None);
    }

    #[test]
    fn age_survives_timer_wrap() {
        let mut tracker = tracker(2);
        tracker.tracks_mut()[0].recognized_ms = Some(u32::MAX - 10);
        tracker.tracks_mut()[1].recognized_ms = Some(5);

        assert_eq!(next_to_recognize(tracker.tracks(), 20, &CONFIG), Some(0));
    }
}
