use std::io::{BufReader, Cursor, ErrorKind};

use serde_json::json;
use tinyasr_wyoming::wyoming::{Event, read_event, write_event};

fn roundtrip(e: &Event) -> Event {
    let mut buf = Vec::new();
    write_event(&mut buf, e).unwrap();
    read_event(&mut BufReader::new(Cursor::new(buf))).unwrap().unwrap()
}

#[test]
fn roundtrip_with_data_and_payload() {
    let mut e = Event::new("audio-chunk", json!({"rate": 16000, "width": 2, "channels": 1}));
    e.payload = vec![1, 2, 3, 4, 5, 6];
    let back = roundtrip(&e);
    assert_eq!(back.kind, "audio-chunk");
    assert_eq!(back.data, e.data);
    assert_eq!(back.payload, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn reads_inline_data_merged_with_data_length() {
    let block = br#"{"language":"ru"}"#;
    let header = format!("{{\"type\":\"transcribe\",\"data\":{{\"a\":1}},\"data_length\":{}}}\n", block.len());
    let mut input = header.into_bytes();
    input.extend_from_slice(block);
    let e = read_event(&mut BufReader::new(Cursor::new(input))).unwrap().unwrap();
    assert_eq!(e.kind, "transcribe");
    assert_eq!(e.data["a"], json!(1));
    assert_eq!(e.data["language"], json!("ru"));
}

#[test]
fn reads_header_without_data_or_payload() {
    let e = read_event(&mut BufReader::new(Cursor::new(b"{\"type\":\"describe\"}\n".to_vec()))).unwrap().unwrap();
    assert_eq!(e.kind, "describe");
    assert!(e.data.is_empty());
    assert!(e.payload.is_empty());
}

#[test]
fn clean_eof_is_none() {
    assert!(read_event(&mut BufReader::new(Cursor::new(Vec::new()))).unwrap().is_none());
}

#[test]
fn truncated_payload_is_error() {
    let mut input = b"{\"type\":\"audio-chunk\",\"payload_length\":10}\n".to_vec();
    input.extend_from_slice(&[0, 1, 2, 3]);
    let err = read_event(&mut BufReader::new(Cursor::new(input))).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
}

#[test]
fn non_ascii_text_is_written_raw() {
    let mut buf = Vec::new();
    write_event(&mut buf, &Event::new("transcript", json!({"text": "Включи свет."}))).unwrap();
    let s = String::from_utf8(buf).unwrap();
    assert!(s.contains("Включи свет."), "{s}");
    assert!(!s.contains("\\u"), "{s}");
}

#[test]
fn oversized_lengths_are_rejected() {
    for field in ["payload_length", "data_length"] {
        let input = format!("{{\"type\":\"audio-chunk\",\"{field}\":1000000000000000}}\n").into_bytes();
        let err = read_event(&mut BufReader::new(Cursor::new(input))).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData, "{field}");
    }
}

#[test]
fn overlong_header_is_rejected() {
    let input = vec![b'{'; 2 << 20];
    let err = read_event(&mut BufReader::new(Cursor::new(input))).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
}
