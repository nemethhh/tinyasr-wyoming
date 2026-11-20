//! Wyoming events: a JSON header line, then optional data JSON (`data_length`) and binary payload (`payload_length`).

use std::io::{self, BufRead, Write};

use serde_json::{Map, Value};

const VERSION: &str = "1.7.2";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Event {
    pub kind: String,
    pub data: Map<String, Value>,
    pub payload: Vec<u8>,
}

impl Event {
    pub fn new(kind: &str, data: Value) -> Event {
        let data = match data {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        Event { kind: kind.to_string(), data, payload: Vec::new() }
    }
}

fn bad(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

fn length(header: &Map<String, Value>, key: &str) -> io::Result<usize> {
    match header.get(key) {
        None | Some(Value::Null) => Ok(0),
        Some(v) => v.as_u64().map(|n| n as usize).ok_or_else(|| bad(format!("bad {key}"))),
    }
}

/// `Ok(None)` on a clean end of stream before a header.
pub fn read_event(r: &mut impl BufRead) -> io::Result<Option<Event>> {
    let mut line = Vec::new();
    if r.read_until(b'\n', &mut line)? == 0 {
        return Ok(None);
    }
    let header: Map<String, Value> = serde_json::from_slice(&line).map_err(|e| bad(format!("header: {e}")))?;
    let kind = header.get("type").and_then(Value::as_str).ok_or_else(|| bad("header without type"))?.to_string();
    let mut data = match header.get("data") {
        Some(Value::Object(m)) => m.clone(),
        _ => Map::new(),
    };
    let data_length = length(&header, "data_length")?;
    if data_length > 0 {
        let mut block = vec![0; data_length];
        r.read_exact(&mut block)?;
        let extra: Map<String, Value> = serde_json::from_slice(&block).map_err(|e| bad(format!("data: {e}")))?;
        data.extend(extra);
    }
    let mut payload = vec![0; length(&header, "payload_length")?];
    r.read_exact(&mut payload)?;
    Ok(Some(Event { kind, data, payload }))
}

pub fn write_event(w: &mut impl Write, e: &Event) -> io::Result<()> {
    let mut header = Map::new();
    header.insert("type".into(), Value::from(e.kind.as_str()));
    header.insert("version".into(), Value::from(VERSION));
    let data = if e.data.is_empty() { Vec::new() } else { serde_json::to_vec(&e.data)? };
    if !data.is_empty() {
        header.insert("data_length".into(), Value::from(data.len()));
    }
    if !e.payload.is_empty() {
        header.insert("payload_length".into(), Value::from(e.payload.len()));
    }
    let mut out = serde_json::to_vec(&header)?;
    out.push(b'\n');
    out.extend_from_slice(&data);
    out.extend_from_slice(&e.payload);
    w.write_all(&out)?;
    w.flush()
}
