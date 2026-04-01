use super::music_fallback_candidates;
use std::path::Path;

#[test]
fn fallback_candidates_follow_ogg_wav_mp3_order() {
    let candidates = music_fallback_candidates(Path::new("assets/audio/ambient"));

    assert!(candidates[0].ends_with("ambient.ogg"));
    assert!(candidates[1].ends_with("ambient.wav"));
    assert!(candidates[2].ends_with("ambient.mp3"));
}
