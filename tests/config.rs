use std::path::PathBuf;

use tinyasr_wyoming::config::Config;
use tinyasr_wyoming::turn::{EndOfSpeech, Mode};

fn parse(args: &[&str]) -> Result<Config, String> {
    Config::from_args(args.iter().map(|s| s.to_string()))
}

#[test]
fn defaults() {
    let c = parse(&[]).unwrap();
    assert_eq!(c.listen, "0.0.0.0:10300");
    assert_eq!(c.models_dir, PathBuf::from("/models"));
    assert_eq!(c.threads, 0);
    assert_eq!(c.turn.beam, 1);
    assert_eq!(c.turn.mode, Mode::Whole);
    assert_eq!(c.turn.end_of_speech, EndOfSpeech::Tinyasr);
    assert_eq!(c.turn.no_speech_seconds, 8.0);
    assert_eq!(c.turn.max_seconds, 30.0);
    assert_eq!(c.turn.min_pause_seconds, 0.7);
    assert_eq!(c.turn.max_pause_seconds, 1.25);
    assert!(!c.turn.debug);
    assert!(c.turn.wake_words.contains(&"alexa".to_string()));
    assert!(c.turn.wake_words.contains(&"алекса".to_string()));
}

#[test]
fn parses_flags() {
    let c = parse(&[
        "--listen", "127.0.0.1:1", "--models", "/m", "--beam", "1", "--threads", "4", "--end-of-speech", "home_assistant",
        "--no-speech-seconds", "5", "--max-seconds", "20", "--min-pause-seconds", "0.5", "--max-pause-seconds", "2",
        "--wake-words", "alexa,echo", "--debug", "--mode", "live",
    ])
    .unwrap();
    assert_eq!(c.listen, "127.0.0.1:1");
    assert_eq!(c.models_dir, PathBuf::from("/m"));
    assert_eq!((c.turn.beam, c.threads), (1, 4));
    assert_eq!(c.turn.end_of_speech, EndOfSpeech::HomeAssistant);
    assert_eq!((c.turn.no_speech_seconds, c.turn.max_seconds), (5.0, 20.0));
    assert_eq!((c.turn.min_pause_seconds, c.turn.max_pause_seconds), (0.5, 2.0));
    assert_eq!(c.turn.wake_words, vec!["alexa", "echo"]);
    assert!(c.turn.debug);
    assert_eq!(c.turn.mode, Mode::Live);
}

#[test]
fn bad_value_names_flag() {
    let e = parse(&["--beam", "x"]).unwrap_err();
    assert!(e.contains("--beam"), "{e}");
    let e = parse(&["--end-of-speech", "maybe"]).unwrap_err();
    assert!(e.contains("--end-of-speech"), "{e}");
    let e = parse(&["--mode", "x"]).unwrap_err();
    assert!(e.contains("--mode"), "{e}");
}

#[test]
fn unknown_flag_named() {
    let e = parse(&["--nope"]).unwrap_err();
    assert!(e.contains("--nope"), "{e}");
}

#[test]
fn missing_value_named() {
    let e = parse(&["--threads"]).unwrap_err();
    assert!(e.contains("--threads"), "{e}");
}

#[test]
fn effective_threads_auto() {
    let auto = parse(&[]).unwrap().effective_threads();
    assert!((1..=8).contains(&auto), "{auto}");
    assert_eq!(parse(&["--threads", "3"]).unwrap().effective_threads(), 3);
}
