mod common;

use common::{models, run, seconds, wav_bytes};
use tinyasr::Word;
use tinyasr_wyoming::turn::{EndOfSpeech, EndReason, Turn, TurnConfig, join_words, normalize_lang};

fn ends_at_pause(fixture: &str, lang: &str, expect: &str) {
    let Some(m) = models() else { return };
    let pcm = wav_bytes(fixture);
    let (o, early) = run(m, lang, &TurnConfig::default(), &pcm, 1024);
    let text = o.text.to_lowercase();
    assert_eq!(o.reason, EndReason::Pause, "{fixture}: {text:?}");
    assert!(early);
    assert!(text.contains(expect), "{fixture}: {text:?}");
    assert!(o.audio_seconds <= seconds(&pcm) - 2.0, "{fixture}: ended at {} of {}", o.audio_seconds, seconds(&pcm));
}

#[test]
fn en_command_ends_at_pause() {
    ends_at_pause("en_command.wav", "en", "kitchen light");
}

#[test]
fn ru_command_ends_at_pause() {
    ends_at_pause("ru_command.wav", "ru", "свет");
}

#[test]
fn uk_command_ends_at_pause() {
    ends_at_pause("uk_command.wav", "uk", "світло");
}

/// The fixtures' speech ends 4 s before the end of the file.
fn speech_end(pcm: &[u8]) -> f32 {
    seconds(pcm) - 4.0
}

#[test]
fn head_end_waits_for_min_pause() {
    let Some(m) = models() else { return };
    let pcm = wav_bytes("ru_command.wav");
    let (o, _) = run(m, "ru", &TurnConfig::default(), &pcm, 1024);
    assert_eq!(o.reason, EndReason::Pause);
    assert!(o.audio_seconds >= speech_end(&pcm) + 0.7, "ended at {} (speech to {})", o.audio_seconds, speech_end(&pcm));
}

#[test]
fn unsure_head_ends_by_max_pause() {
    let Some(m) = models() else { return };
    let pcm = wav_bytes("en_command.wav");
    let (o, _) = run(m, "en", &TurnConfig::default(), &pcm, 1024);
    assert_eq!(o.reason, EndReason::Pause);
    assert!(o.audio_seconds <= speech_end(&pcm) + 1.25 + 0.3, "ended at {} (speech to {})", o.audio_seconds, speech_end(&pcm));
}

#[test]
fn min_pause_is_configurable() {
    let Some(m) = models() else { return };
    let pcm = wav_bytes("ru_command.wav");
    let cfg = TurnConfig { min_pause_seconds: 1.0, max_pause_seconds: 2.0, ..TurnConfig::default() };
    let (o, _) = run(m, "ru", &cfg, &pcm, 1024);
    assert_eq!(o.reason, EndReason::Pause);
    assert!(o.audio_seconds >= speech_end(&pcm) + 1.0, "ended at {}", o.audio_seconds);
}

#[test]
fn mid_sentence_pause_does_not_cut() {
    let Some(m) = models() else { return };
    let (o, _) = run(m, "en", &TurnConfig::default(), &wav_bytes("en_midpause.wav"), 1024);
    assert_eq!(o.reason, EndReason::Pause, "{:?}", o.text);
    assert!(o.text.to_lowercase().contains("ten minutes"), "{:?}", o.text);
}

#[test]
fn wake_word_alone_does_not_end() {
    let Some(m) = models() else { return };
    let (o, _) = run(m, "en", &TurnConfig::default(), &wav_bytes("en_wake_pause.wav"), 1024);
    assert_eq!(o.reason, EndReason::Pause, "{:?}", o.text);
    assert!(o.text.to_lowercase().contains("time"), "{:?}", o.text);
}

#[test]
fn silence_is_no_speech() {
    let Some(m) = models() else { return };
    let (o, early) = run(m, "en", &TurnConfig::default(), &wav_bytes("silence.wav"), 1024);
    assert_eq!(o.reason, EndReason::NoSpeech);
    assert!(early);
    assert_eq!(o.text, "");
    assert!((8.0..8.2).contains(&o.audio_seconds), "{}", o.audio_seconds);
}

#[test]
fn max_length_cuts() {
    let Some(m) = models() else { return };
    let cfg = TurnConfig { max_seconds: 2.0, ..TurnConfig::default() };
    let (o, early) = run(m, "en", &cfg, &wav_bytes("en_command.wav"), 1024);
    assert_eq!(o.reason, EndReason::MaxLength, "{:?}", o.text);
    assert!(early);
}

#[test]
fn home_assistant_mode_waits_for_stop() {
    let Some(m) = models() else { return };
    let cfg = TurnConfig { end_of_speech: EndOfSpeech::HomeAssistant, ..TurnConfig::default() };
    let mut turn = Turn::new(m, "en", &cfg);
    for c in wav_bytes("en_command.wav").chunks(1024) {
        assert!(turn.push_bytes(c).is_none());
    }
    let o = turn.finish();
    assert_eq!(o.reason, EndReason::AudioStop);
    assert!(o.text.to_lowercase().contains("kitchen light"), "{:?}", o.text);
}

#[test]
fn odd_byte_chunks_give_same_text() {
    let Some(m) = models() else { return };
    let cfg = TurnConfig { end_of_speech: EndOfSpeech::HomeAssistant, ..TurnConfig::default() };
    let pcm = wav_bytes("en_command.wav");
    let (even, _) = run(m, "en", &cfg, &pcm, 1024);
    let (odd, _) = run(m, "en", &cfg, &pcm, 1023);
    assert!(!even.text.is_empty());
    assert_eq!(odd.text, even.text);
}

#[test]
fn normalize_lang_cases() {
    assert_eq!(normalize_lang("ru-RU"), "ru");
    assert_eq!(normalize_lang("ru_RU"), "ru");
    assert_eq!(normalize_lang("RU"), "ru");
    assert_eq!(normalize_lang("en"), "en");
}

#[test]
fn join_words_respects_join() {
    let w = |t: &str, join| Word { text: t.into(), start: 0.0, end: 0.0, join };
    assert_eq!(join_words(&[w("Hello,", false), w("world.", false)]), "Hello, world.");
    assert_eq!(join_words(&[w("ab", false), w("cd", true), w("ef", false)]), "abcd ef");
    assert_eq!(join_words(&[]), "");
}
