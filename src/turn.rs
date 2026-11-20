//! One request: a live tinyasr session plus the rules that end it, or audio buffered for one pass at the end.

use tinyasr::{Event, Models, Session, Word};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndOfSpeech {
    Tinyasr,
    HomeAssistant,
}

/// `Whole`: buffer, one pass at the end (HA ends the request; slow CPUs). `Live`: stream, tinyasr may end it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Whole,
    Live,
}

#[derive(Clone, Debug)]
pub struct TurnConfig {
    pub mode: Mode,
    pub end_of_speech: EndOfSpeech,
    pub beam: usize,
    pub no_speech_seconds: f32,
    pub max_seconds: f32,
    /// tinyasr's "finished" counts only after this much silence (HA's own default end of speech).
    pub min_pause_seconds: f32,
    /// After speech, this much silence ends the request whatever tinyasr's end-of-turn head says.
    pub max_pause_seconds: f32,
    pub wake_words: Vec<String>,
    pub debug: bool,
}

impl Default for TurnConfig {
    fn default() -> TurnConfig {
        TurnConfig {
            mode: Mode::Whole,
            end_of_speech: EndOfSpeech::Tinyasr,
            beam: 1,
            no_speech_seconds: 8.0,
            max_seconds: 30.0,
            min_pause_seconds: 0.7,
            max_pause_seconds: 1.25,
            wake_words: ["alexa", "алекса", "echo", "эхо", "computer", "компьютер", "amazon", "амазон", "ziggy"]
                .map(String::from)
                .to_vec(),
            debug: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    Pause,
    NoSpeech,
    MaxLength,
    AudioStop,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub text: String,
    pub reason: EndReason,
    pub audio_seconds: f32,
}

enum Engine<'m> {
    Live(Session<'m>),
    Whole { models: &'m Models, lang: String, audio: Vec<f32> },
}

pub struct Turn<'m> {
    engine: Engine<'m>,
    cfg: TurnConfig,
    wake: Vec<String>,
    carry: Option<u8>,
    samples: u64,
    committed: Vec<Word>,
    heard: bool,
    pause_ends: u32,
    head_done: bool,
}

/// `"ru-RU"`, `"ru_RU"`, `"RU"` → `"ru"`.
pub fn normalize_lang(lang: &str) -> String {
    lang.split(['-', '_']).next().unwrap_or("").trim().to_lowercase()
}

pub fn join_words(words: &[Word]) -> String {
    let mut s = String::new();
    for w in words {
        if !s.is_empty() && !w.join {
            s.push(' ');
        }
        s.push_str(&w.text);
    }
    s
}

/// Lowercase, letters and digits only, single spaces.
fn normalize_text(s: &str) -> String {
    let s: String = s.chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect();
    s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

impl<'m> Turn<'m> {
    pub fn new(models: &'m Models, lang: &str, cfg: &TurnConfig) -> Turn<'m> {
        let engine = match cfg.mode {
            Mode::Live => {
                Engine::Live(Session::new(models, &[lang], tinyasr::Options { beam: cfg.beam, ..Default::default() }))
            }
            Mode::Whole => Engine::Whole { models, lang: lang.to_string(), audio: Vec::new() },
        };
        Turn {
            engine,
            cfg: cfg.clone(),
            wake: cfg.wake_words.iter().map(|w| normalize_text(w)).collect(),
            carry: None,
            samples: 0,
            committed: Vec::new(),
            heard: false,
            pause_ends: 0,
            head_done: false,
        }
    }

    fn audio_seconds(&self) -> f32 {
        self.samples as f32 / tinyasr::SAMPLE_RATE as f32
    }

    /// Words beyond a wake word alone.
    fn is_speech(&self, words: &[Word]) -> bool {
        let t = normalize_text(&join_words(words));
        !t.is_empty() && !self.wake.contains(&t)
    }

    fn take(&mut self, events: Vec<Event>) {
        for e in events {
            match e {
                Event::Commit(words) => {
                    self.committed.extend(words);
                    self.heard |= self.is_speech(&self.committed);
                }
                Event::Partial { open, tail } => {
                    let words: Vec<Word> = open.into_iter().chain(tail).collect();
                    self.heard |= self.is_speech(&words);
                }
                Event::Eou { at, p } if self.cfg.debug => eprintln!("end of turn head at {at:.2} s: {p:.2}"),
                _ => {}
            }
        }
    }

    fn conclude(&mut self, reason: EndReason) -> Outcome {
        let text = match &mut self.engine {
            Engine::Live(session) => {
                let events = session.finish();
                self.take(events);
                join_words(&self.committed)
            }
            Engine::Whole { models, lang, audio } => whole_pass(models, audio, lang, self.cfg.beam),
        };
        Outcome { text, reason, audio_seconds: self.audio_seconds() }
    }

    /// 16-bit little-endian PCM, any length (an odd last byte waits for the next call).
    pub fn push_bytes(&mut self, pcm: &[u8]) -> Option<Outcome> {
        let mut bytes = Vec::with_capacity(pcm.len() + 1);
        bytes.extend(self.carry.take());
        bytes.extend_from_slice(pcm);
        if bytes.len() % 2 == 1 {
            self.carry = bytes.pop();
        }
        let x: Vec<f32> =
            bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0).collect();
        self.samples += x.len() as u64;
        let session = match &mut self.engine {
            Engine::Live(session) => session,
            Engine::Whole { audio, .. } => {
                audio.extend_from_slice(&x);
                return (self.audio_seconds() >= self.cfg.max_seconds).then(|| self.conclude(EndReason::MaxLength));
            }
        };
        let events = session.push(&x);
        let (pause_ends, quiet) = (session.pause_ends(), session.quiet_seconds());
        self.take(events);

        if self.cfg.end_of_speech == EndOfSpeech::Tinyasr {
            if pause_ends > self.pause_ends {
                self.pause_ends = pause_ends;
                self.head_done |= self.is_speech(&self.committed);
            }
            if quiet == 0.0 {
                self.head_done = false;
            }
            if (self.head_done && quiet >= self.cfg.min_pause_seconds) || (self.heard && quiet >= self.cfg.max_pause_seconds) {
                return Some(self.conclude(EndReason::Pause));
            }
        }
        if !self.heard && self.audio_seconds() >= self.cfg.no_speech_seconds {
            return Some(Outcome { text: String::new(), reason: EndReason::NoSpeech, audio_seconds: self.audio_seconds() });
        }
        if self.audio_seconds() >= self.cfg.max_seconds {
            return Some(self.conclude(EndReason::MaxLength));
        }
        None
    }

    pub fn finish(mut self) -> Outcome {
        self.conclude(EndReason::AudioStop)
    }
}

/// One full-context pass; a beam search that finds no words is retried greedy (it can miss a clear sentence).
fn whole_pass(models: &Models, audio: &[f32], lang: &str, beam: usize) -> String {
    let run = |beam| join_words(&tinyasr::Transcription::new(models, audio, &[lang], beam).flatten().collect::<Vec<_>>());
    let text = run(beam);
    if text.is_empty() && beam > 1 { run(1) } else { text }
}
