//! Chip suggestions for the stack composer (D-62): what code keeps of Claude's answer.

use serde_json::json;
use shotpilot::suggest::{tidy, Ask, MAX_CHIPS};

fn ask(target: &str, stack: &[&str]) -> Ask {
    Ask { target: target.into(), stack: stack.iter().map(|s| s.to_string()).collect(), ..Default::default() }
}

#[test]
fn chips_are_tidied_and_never_repeat_the_stack() {
    let raw = json!({"chips": ["1. もう少し低い位置から。", "ゆっくり前進", "・ゆっくり 前進", "30秒に収める",
        "「霧」", "見上げる", "被写体を右3分の1に", "とても長い指示とても長い指示とても長い指示とても長い指示とても長い指示とても長い指示",
        "逆光", "夜にする"]});
    let out = tidy(&raw, &ask("row", &["見上げる"]));
    // a number in front and a full stop go; a repeat, a stacked chip and an essay are dropped; at most six
    assert_eq!(out, vec!["もう少し低い位置から", "ゆっくり前進", "30秒に収める", "霧", "被写体を右3分の1に", "逆光"]);
    assert_eq!(out.len(), MAX_CHIPS);
    // words to burn in keep their quotes
    let raw = json!({"chips": ["字幕「扉は閉まらない」を重ねる"]});
    assert_eq!(tidy(&raw, &ask("row", &[])), vec!["字幕「扉は閉まらない」を重ねる"]);
}

#[test]
fn mood_words_stay_short() {
    let raw = json!({"chips": ["霧", "冷たい青と提灯の赤がにじむ雨上がりの路地裏の空気", 3, null]});
    assert_eq!(tidy(&raw, &ask("mood", &[])), vec!["霧"]);
    assert!(tidy(&json!({"nothing": true}), &ask("pitch", &[])).is_empty());
}
