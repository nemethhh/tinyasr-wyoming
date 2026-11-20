mod common;

use std::io::{BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::OnceLock;
use std::time::Duration;

use common::{models, wav_bytes};
use serde_json::{Value, json};
use tinyasr_wyoming::config::Config;
use tinyasr_wyoming::server::{info_event, load_models, serve};
use tinyasr_wyoming::turn::EndOfSpeech;
use tinyasr_wyoming::wyoming::{Event, read_event, write_event};

fn start(args: &[&str]) -> Option<String> {
    let m = models()?;
    let cfg: &'static Config = Box::leak(Box::new(Config::from_args(args.iter().map(|s| s.to_string())).unwrap()));
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap().to_string();
    std::thread::spawn(move || serve(l, m, cfg));
    Some(addr)
}

/// A live-mode server shared by the tests, or None without models.
fn server() -> Option<String> {
    static ADDR: OnceLock<Option<String>> = OnceLock::new();
    ADDR.get_or_init(|| start(&["--mode", "live"])).clone()
}

/// A whole-mode (default) server.
fn whole_server() -> Option<String> {
    static ADDR: OnceLock<Option<String>> = OnceLock::new();
    ADDR.get_or_init(|| start(&[])).clone()
}

struct Client {
    w: TcpStream,
    r: BufReader<TcpStream>,
}

impl Client {
    fn connect(addr: &str) -> Client {
        let w = TcpStream::connect(addr).unwrap();
        w.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
        Client { r: BufReader::new(w.try_clone().unwrap()), w }
    }
    fn send(&mut self, kind: &str, data: Value) {
        write_event(&mut self.w, &Event::new(kind, data)).unwrap();
    }
    fn start(&mut self, lang: &str) {
        self.send("transcribe", json!({"language": lang}));
        self.send("audio-start", json!({"rate": 16000, "width": 2, "channels": 1}));
    }
    fn chunks(&mut self, pcm: &[u8]) {
        for c in pcm.chunks(1024) {
            let mut e = Event::new("audio-chunk", json!({"rate": 16000, "width": 2, "channels": 1}));
            e.payload = c.to_vec();
            if write_event(&mut self.w, &e).is_err() {
                return;
            }
        }
    }
    fn next(&mut self) -> Option<Event> {
        read_event(&mut self.r).unwrap()
    }
}

fn text(e: &Event) -> String {
    e.data["text"].as_str().unwrap().to_lowercase()
}

#[test]
fn describe_returns_info() {
    let Some(addr) = server() else { return };
    let mut c = Client::connect(&addr);
    c.send("describe", json!({}));
    let e = c.next().unwrap();
    assert_eq!(e.kind, "info");
    let asr = &e.data["asr"][0];
    assert_eq!(asr["name"], json!("tinyasr"));
    assert_eq!(asr["models"][0]["languages"], json!(["en", "ru", "uk"]));
    assert_eq!(asr["requires_external_vad"], json!(false));
    assert_eq!(asr["supports_transcript_streaming"], json!(false));
}

#[test]
fn info_in_home_assistant_mode_requires_external_vad() {
    let cfg = Config::from_args(["--end-of-speech", "home_assistant"].map(String::from).into_iter()).unwrap();
    assert_eq!(cfg.turn.end_of_speech, EndOfSpeech::HomeAssistant);
    let e = info_event(&cfg, &["en", "ru", "uk"]);
    assert_eq!(e.data["asr"][0]["requires_external_vad"], json!(true));
}

#[test]
fn transcript_arrives_before_audio_stop() {
    let Some(addr) = server() else { return };
    let mut c = Client::connect(&addr);
    c.start("en");
    c.chunks(&wav_bytes("en_command.wav"));
    // no audio-stop: the server ends the request itself
    let e = c.next().unwrap();
    assert_eq!(e.kind, "transcript");
    assert!(text(&e).contains("kitchen light"), "{:?}", e.data);
    assert_eq!(e.data["language"], json!("en"));
}

#[test]
fn regional_language_code_accepted() {
    let Some(addr) = server() else { return };
    let mut c = Client::connect(&addr);
    c.start("ru-RU");
    c.chunks(&wav_bytes("ru_command.wav"));
    c.send("audio-stop", json!({}));
    let e = c.next().unwrap();
    assert_eq!(e.kind, "transcript");
    assert!(text(&e).contains("свет"), "{:?}", e.data);
}

#[test]
fn unsupported_language_is_error() {
    let Some(addr) = server() else { return };
    let mut c = Client::connect(&addr);
    c.send("transcribe", json!({"language": "de"}));
    let e = c.next().unwrap();
    assert_eq!(e.kind, "error");
    assert_eq!(e.data["code"], json!("unsupported-language"));
    assert!(c.next().is_none());
}

#[test]
fn unsupported_format_is_error() {
    let Some(addr) = server() else { return };
    let mut c = Client::connect(&addr);
    c.send("transcribe", json!({"language": "en"}));
    c.send("audio-start", json!({"rate": 22050, "width": 2, "channels": 1}));
    let e = c.next().unwrap();
    assert_eq!(e.kind, "error");
    assert_eq!(e.data["code"], json!("unsupported-format"));
    assert!(c.next().is_none());
}

#[test]
fn only_one_transcript_per_request() {
    let Some(addr) = server() else { return };
    let mut c = Client::connect(&addr);
    c.start("en");
    c.chunks(&wav_bytes("en_command.wav"));
    c.chunks(&wav_bytes("en_command.wav"));
    c.send("audio-stop", json!({}));
    c.w.shutdown(Shutdown::Write).unwrap();
    let mut transcripts = 0;
    while let Some(e) = c.next() {
        transcripts += (e.kind == "transcript") as usize;
    }
    assert_eq!(transcripts, 1);
}

#[test]
fn client_closing_early_does_not_stop_server() {
    let Some(addr) = server() else { return };
    {
        let mut c = Client::connect(&addr);
        c.start("en");
        let pcm = wav_bytes("en_command.wav");
        c.chunks(&pcm[..pcm.len() / 2]);
        // dropped mid-upload, as HA does when it cancels
    }
    {
        let mut c = Client::connect(&addr);
        c.start("en");
        c.chunks(&wav_bytes("en_command.wav"));
        assert_eq!(c.next().unwrap().kind, "transcript");
        // dropped right after the transcript
    }
    let mut c = Client::connect(&addr);
    c.send("describe", json!({}));
    assert_eq!(c.next().unwrap().kind, "info");
}

#[test]
fn two_requests_at_once() {
    let Some(addr) = server() else { return };
    let one = |lang: &'static str, fixture: &'static str, addr: String| {
        std::thread::spawn(move || {
            let mut c = Client::connect(&addr);
            c.start(lang);
            c.chunks(&wav_bytes(fixture));
            c.send("audio-stop", json!({}));
            text(&c.next().unwrap())
        })
    };
    let en = one("en", "en_command.wav", addr.clone());
    let ru = one("ru", "ru_command.wav", addr);
    let (en, ru) = (en.join().unwrap(), ru.join().unwrap());
    assert!(en.contains("kitchen light"), "{en}");
    assert!(ru.contains("свет"), "{ru}");
}

#[test]
fn load_models_missing_file_names_it() {
    let dir = std::env::temp_dir().join(format!("tinyasr-wyoming-empty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let e = load_models(&dir, 1).err().unwrap();
    assert!(e.contains("en-ru-uk.tzf"), "{e}");
}

#[test]
fn load_models_corrupt_file_names_it() {
    let dir = std::env::temp_dir().join(format!("tinyasr-wyoming-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::File::create(dir.join("en-ru-uk.tzf")).unwrap().write_all(b"not a model").unwrap();
    std::fs::File::create(dir.join("silero_vad.onnx")).unwrap().write_all(b"not a model").unwrap();
    let e = load_models(&dir, 1).err().unwrap();
    assert!(e.contains("en-ru-uk.tzf") || e.contains("silero_vad.onnx"), "{e}");
}

#[test]
fn whole_mode_server_requires_external_vad() {
    let Some(addr) = whole_server() else { return };
    let mut c = Client::connect(&addr);
    c.send("describe", json!({}));
    assert_eq!(c.next().unwrap().data["asr"][0]["requires_external_vad"], json!(true));
}

#[test]
fn whole_mode_answers_at_audio_stop() {
    let Some(addr) = whole_server() else { return };
    let mut c = Client::connect(&addr);
    c.start("uk-UA");
    c.chunks(&wav_bytes("uk_command.wav"));
    c.send("audio-stop", json!({}));
    let e = c.next().unwrap();
    assert_eq!(e.kind, "transcript");
    assert!(text(&e).contains("світло"), "{:?}", e.data);
}
