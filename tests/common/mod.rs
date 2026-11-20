#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use tinyasr_wyoming::turn::{Outcome, Turn, TurnConfig};

pub fn models_dir() -> PathBuf {
    std::env::var_os("TINYASR_MODELS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models"))
}

/// The models, loaded once; `None` (the test skips) when the files are absent.
pub fn models() -> Option<&'static tinyasr::Models> {
    static M: OnceLock<Option<tinyasr::Models>> = OnceLock::new();
    let m = M
        .get_or_init(|| {
            let dir = models_dir();
            let asr = std::fs::read(dir.join("en-ru-uk.tzf")).ok()?;
            let vad = std::fs::read(dir.join("silero_vad.onnx")).ok()?;
            Some(tinyasr::Models::load(tinyasr::Files { asr: &asr, vad: &vad }).expect("models load"))
        })
        .as_ref();
    if m.is_none() {
        eprintln!("skipped: no models in {}", models_dir().display());
    }
    m
}

/// PCM bytes of a fixture (the WAV's data chunk).
pub fn wav_bytes(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    let b = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut i = 12;
    while i + 8 <= b.len() {
        let size = u32::from_le_bytes(b[i + 4..i + 8].try_into().unwrap()) as usize;
        if &b[i..i + 4] == b"data" {
            return b[i + 8..(i + 8 + size).min(b.len())].to_vec();
        }
        i += 8 + size + (size & 1);
    }
    panic!("{name}: no data chunk");
}

pub fn seconds(pcm: &[u8]) -> f32 {
    pcm.len() as f32 / 32000.0
}

/// Push `pcm` in `chunk`-byte pieces; the outcome and whether it came before the audio ran out.
pub fn run(models: &tinyasr::Models, lang: &str, cfg: &TurnConfig, pcm: &[u8], chunk: usize) -> (Outcome, bool) {
    let mut turn = Turn::new(models, lang, cfg);
    for c in pcm.chunks(chunk) {
        if let Some(o) = turn.push_bytes(c) {
            return (o, true);
        }
    }
    (turn.finish(), false)
}
