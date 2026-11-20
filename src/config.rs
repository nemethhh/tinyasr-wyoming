//! Command-line settings (the add-on's run.sh maps its options to these flags).

use std::path::PathBuf;
use std::str::FromStr;

use crate::turn::{EndOfSpeech, Mode, TurnConfig};

#[derive(Clone, Debug)]
pub struct Config {
    pub listen: String,
    pub asr_file: PathBuf,
    pub vad_file: PathBuf,
    pub threads: usize,
    pub turn: TurnConfig,
}

fn num<T: FromStr>(flag: &str, v: &str) -> Result<T, String> {
    v.parse().map_err(|_| format!("{flag}: bad value {v:?}"))
}

impl Config {
    /// Arguments without the program name.
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Config, String> {
        let mut c = Config {
            listen: "0.0.0.0:10300".into(),
            asr_file: PathBuf::new(),
            vad_file: PathBuf::new(),
            threads: 0,
            turn: TurnConfig::default(),
        };
        let mut args = args;
        let (mut models_dir, mut asr, mut vad) = (PathBuf::from("/models"), None, None);
        while let Some(flag) = args.next() {
            if flag == "--debug" {
                c.turn.debug = true;
                continue;
            }
            let known = [
                "--listen", "--mode", "--models", "--asr", "--vad", "--beam", "--threads", "--end-of-speech", "--no-speech-seconds", "--max-seconds",
                "--min-pause-seconds", "--max-pause-seconds", "--wake-words",
            ];
            if !known.contains(&flag.as_str()) {
                return Err(format!("unknown flag {flag}"));
            }
            let v = args.next().ok_or_else(|| format!("{flag}: missing value"))?;
            let f = flag.as_str();
            match f {
                "--listen" => c.listen = v,
                "--models" => models_dir = PathBuf::from(v),
                "--asr" => asr = Some(PathBuf::from(v)),
                "--vad" => vad = Some(PathBuf::from(v)),
                "--mode" => {
                    c.turn.mode = match v.as_str() {
                        "whole" => Mode::Whole,
                        "live" => Mode::Live,
                        _ => return Err(format!("{f}: bad value {v:?} (whole or live)")),
                    }
                }
                "--beam" => c.turn.beam = num(f, &v)?,
                "--threads" => c.threads = num(f, &v)?,
                "--end-of-speech" => {
                    c.turn.end_of_speech = match v.as_str() {
                        "tinyasr" => EndOfSpeech::Tinyasr,
                        "home_assistant" => EndOfSpeech::HomeAssistant,
                        _ => return Err(format!("{f}: bad value {v:?} (tinyasr or home_assistant)")),
                    }
                }
                "--no-speech-seconds" => c.turn.no_speech_seconds = num(f, &v)?,
                "--max-seconds" => c.turn.max_seconds = num(f, &v)?,
                "--min-pause-seconds" => c.turn.min_pause_seconds = num(f, &v)?,
                "--max-pause-seconds" => c.turn.max_pause_seconds = num(f, &v)?,
                "--wake-words" => {
                    c.turn.wake_words = v.split(',').map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect()
                }
                _ => unreachable!(),
            }
        }
        c.asr_file = asr.unwrap_or_else(|| models_dir.join("en-ru-uk.tzf"));
        c.vad_file = vad.unwrap_or_else(|| models_dir.join("silero_vad.onnx"));
        if c.turn.beam == 0 {
            return Err("--beam: must be at least 1".into());
        }
        Ok(c)
    }

    /// `threads`, or with 0 the machine's cores (at most 8).
    pub fn effective_threads(&self) -> usize {
        match self.threads {
            0 => std::thread::available_parallelism().map_or(1, |n| n.get()).min(8),
            n => n,
        }
    }
}
