//! The Wyoming speech-to-text service: one thread per connection, one request per connection (as HA uses it).

use std::io::{self, BufReader};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::time::Instant;

use serde_json::json;
use tinyasr::Models;

use crate::config::Config;
use crate::turn::{EndOfSpeech, Outcome, Turn, normalize_lang};
use crate::wyoming::{Event, read_event, write_event};

const ASR_FILE: &str = "en-ru-uk.tzf";
const VAD_FILE: &str = "silero_vad.onnx";
const MODEL_NAME: &str = "en-ru-uk";

pub fn load_models(dir: &Path, threads: usize) -> Result<Models, String> {
    let read = |f: &str| std::fs::read(dir.join(f)).map_err(|e| format!("{}: {e}", dir.join(f).display()));
    let (asr, vad) = (read(ASR_FILE)?, read(VAD_FILE)?);
    let mut m = Models::load(tinyasr::Files { asr: &asr, vad: &vad })
        .map_err(|e| format!("{ASR_FILE} / {VAD_FILE} in {}: {e}", dir.display()))?;
    m.set_threads(threads);
    Ok(m)
}

pub fn info_event(cfg: &Config, langs: &[&str]) -> Event {
    let attribution = json!({"name": "tinyasr", "url": "https://github.com/dkaraush/tinyasr"});
    Event::new(
        "info",
        json!({"asr": [{
            "name": "tinyasr",
            "description": "tinyasr speech to text",
            "attribution": attribution,
            "installed": true,
            "version": env!("CARGO_PKG_VERSION"),
            "models": [{
                "name": MODEL_NAME,
                "description": "tinyasr en-ru-uk",
                "attribution": attribution,
                "installed": true,
                "version": tinyasr::version().to_string(),
                "languages": langs,
            }],
            "supports_transcript_streaming": false,
            "requires_external_vad": cfg.turn.end_of_speech == EndOfSpeech::HomeAssistant,
            "prefers_auto_gain_enabled": true,
            "prefers_noise_reduction_enabled": true,
        }]}),
    )
}

fn error_event(code: &str, text: String) -> Event {
    Event::new("error", json!({"text": text, "code": code}))
}

pub fn serve(listener: TcpListener, models: &'static Models, cfg: &'static Config) {
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(move || {
                    let peer = s.peer_addr().map(|a| a.to_string()).unwrap_or_default();
                    if let Err(e) = handle(s, models, cfg) {
                        eprintln!("{peer}: connection ended: {e}");
                    }
                });
            }
            Err(e) => eprintln!("accept: {e}"),
        }
    }
}

fn handle(stream: TcpStream, models: &'static Models, cfg: &'static Config) -> io::Result<()> {
    let mut w = stream.try_clone()?;
    let mut r = BufReader::new(stream);
    let langs = models.langs();
    let mut lang: Option<String> = None;
    let mut turn: Option<Turn> = None;
    let mut answered = false;
    let mut last_chunk = Instant::now();

    let send_transcript = |w: &mut TcpStream, o: Outcome, lang: &str, since: Instant| -> io::Result<()> {
        eprintln!(
            "request: {lang}, {:.2} s of audio, end: {:?}, {} ms after the last chunk{}",
            o.audio_seconds,
            o.reason,
            since.elapsed().as_millis(),
            if cfg.turn.debug { format!(": {:?}", o.text) } else { String::new() }
        );
        write_event(w, &Event::new("transcript", json!({"text": o.text, "language": lang})))
    };

    while let Some(e) = read_event(&mut r)? {
        if answered {
            continue; // HA may still be uploading; one transcript per request
        }
        match e.kind.as_str() {
            "describe" => write_event(&mut w, &info_event(cfg, &langs))?,
            "transcribe" => {
                let l = normalize_lang(e.data.get("language").and_then(|v| v.as_str()).unwrap_or(""));
                if !langs.contains(&l.as_str()) {
                    write_event(&mut w, &error_event("unsupported-language", format!("language {l:?} not in {langs:?}")))?;
                    return Ok(());
                }
                lang = Some(l);
            }
            "audio-start" => {
                let field = |k: &str| e.data.get(k).and_then(|v| v.as_u64());
                if (field("rate"), field("width"), field("channels")) != (Some(16000), Some(2), Some(1)) {
                    let text = format!("audio must be 16000 Hz, 16 bit, mono; got {:?}", e.data);
                    write_event(&mut w, &error_event("unsupported-format", text))?;
                    return Ok(());
                }
                let l = lang.get_or_insert_with(|| {
                    eprintln!("no language given: {}", langs[0]);
                    langs[0].to_string()
                });
                turn = Some(Turn::new(models, l, &cfg.turn));
            }
            "audio-chunk" => {
                let Some(t) = turn.as_mut() else { continue };
                last_chunk = Instant::now();
                if let Some(o) = t.push_bytes(&e.payload) {
                    answered = true;
                    send_transcript(&mut w, o, lang.as_deref().unwrap_or_default(), last_chunk)?;
                }
            }
            "audio-stop" => {
                if let Some(t) = turn.take() {
                    answered = true;
                    send_transcript(&mut w, t.finish(), lang.as_deref().unwrap_or_default(), last_chunk)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
