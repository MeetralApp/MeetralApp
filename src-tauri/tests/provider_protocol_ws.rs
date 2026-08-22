//! Integration test: provider protocol frames through the shared live WS decode path.
//! Mock level — canned JSON frames + one loopback WS server; no real cloud calls.

use futures_util::{SinkExt, StreamExt};
use meetral_lib::providers::gemini::protocol::parse_server_message;
use meetral_lib::providers::openai::protocol::{parse_server_event, PhraseCommitter};
use meetral_lib::providers::shared::live::decode_ws_message;
use meetral_lib::providers::soniox::protocol::{parse_stt_message, SonioxTokenAccumulator};
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

#[test]
fn gemini_frames_flow_through_shared_decode() {
    let setup_text = decode_ws_message(Message::Binary(br#"{"setupComplete": {}}"#.to_vec()))
        .expect("binary setup frame decodes");
    let (setup, audio, transcripts, turn_complete) = parse_server_message(&setup_text, "outbound");
    assert!(setup);
    assert!(audio.is_empty());
    assert!(transcripts.is_empty());
    assert!(!turn_complete);

    let turn = r#"{
        "serverContent": {
            "inputTranscription": {"text": "hello"},
            "outputTranscription": {"text": "xin chào"},
            "turnComplete": true
        }
    }"#;
    let text = decode_ws_message(Message::Text(turn.into())).expect("text frame decodes");
    let (setup, _, transcripts, turn_complete) = parse_server_message(&text, "outbound");
    assert!(!setup);
    assert!(turn_complete);
    assert_eq!(transcripts.len(), 1);
    assert_eq!(transcripts[0].source_text.as_deref(), Some("hello"));
    assert_eq!(transcripts[0].translated_text.as_deref(), Some("xin chào"));
    assert!(transcripts[0].turn_complete);

    assert!(decode_ws_message(Message::Ping(Vec::new())).is_none());
}

#[test]
fn openai_events_accumulate_through_shared_decode() {
    let mut committer = PhraseCommitter::default();

    let delta_text = decode_ws_message(Message::Text(
        r#"{"type":"conversation.item.input_audio_transcription.delta","delta":"Xin "}"#.into(),
    ))
    .expect("delta frame decodes");
    let value: Value = serde_json::from_str(&delta_text).unwrap();
    let (_, _, interim, _) = parse_server_event(&value, "outbound", &mut committer);
    assert!(interim.iter().any(|e| e.interim));

    let done_text = decode_ws_message(Message::Text(
        r#"{"type":"conversation.item.input_audio_transcription.completed","transcript":"Xin chào mọi người."}"#
            .into(),
    ))
    .expect("done frame decodes");
    let value: Value = serde_json::from_str(&done_text).unwrap();
    let (_, _, events, _) = parse_server_event(&value, "outbound", &mut committer);
    let complete = events
        .iter()
        .find(|e| e.turn_complete)
        .expect("turn complete");
    assert_eq!(complete.source_text.as_deref(), Some("Xin chào mọi người."));
}

#[test]
fn soniox_tokens_commit_turn_through_shared_decode() {
    let mut acc = SonioxTokenAccumulator::default();

    let interim = decode_ws_message(Message::Text(
        r#"{"tokens":[{"text":"Xin","translation_status":"original","is_final":false},{"text":"Hello","translation_status":"translation","is_final":false}]}"#.into(),
    ))
    .expect("interim frame decodes");
    let events = parse_stt_message(&interim, "outbound", &mut acc);
    assert_eq!(events.len(), 1);
    assert!(events[0].interim);

    let fin = decode_ws_message(Message::Text(
        r#"{"tokens":[{"text":"Xin","translation_status":"original","is_final":true},{"text":"Hello","translation_status":"translation","is_final":true},{"text":"<end>","is_final":true}]}"#.into(),
    ))
    .expect("final frame decodes");
    let events = parse_stt_message(&fin, "outbound", &mut acc);
    assert_eq!(events.len(), 1);
    assert!(events[0].turn_complete);
    assert_eq!(events[0].source_text.as_deref(), Some("Xin"));
    assert_eq!(events[0].translated_text.as_deref(), Some("Hello"));
}

/// One loopback WS server — proves the shared decode path works on real socket
/// framing without any cloud dependency. Uses a sentinel frame instead of the
/// WS close handshake: closing mid-read can surface as ConnectionAborted on
/// Windows loopback, while the sentinel gives a deterministic end-of-stream.
#[tokio::test]
async fn loopback_ws_server_frames_reach_shared_decode() {
    const DONE: &str = r#"{"__done__":true}"#;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local addr");

    let frames = vec![
        r#"{"setupComplete": {}}"#.to_string(),
        r#"{"serverContent": {"outputTranscription": {"text": "xin chào"}, "turnComplete": true}}"#
            .to_string(),
    ];

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("ws handshake");
        for frame in frames {
            ws.send(Message::Text(frame)).await.expect("send frame");
        }
        ws.send(Message::Text(DONE.into()))
            .await
            .expect("send sentinel");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next()).await;
    });

    let url = format!("ws://{addr}");
    let (mut ws, _) = tokio_tungstenite::connect_async(url.as_str())
        .await
        .expect("connect");

    let mut texts = Vec::new();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(msg) = ws.next().await {
            let msg = msg.expect("ws message");
            if let Some(text) = decode_ws_message(msg) {
                if text == DONE {
                    break;
                }
                texts.push(text);
            }
        }
        drop(ws);
    })
    .await
    .expect("client read loop timed out");
    server.await.expect("server task");

    assert_eq!(texts.len(), 2);
    let (setup, _, _, _) = parse_server_message(&texts[0], "inbound");
    assert!(setup);
    let (_, _, transcripts, turn_complete) = parse_server_message(&texts[1], "inbound");
    assert!(turn_complete);
    assert_eq!(transcripts.len(), 1);
    assert_eq!(transcripts[0].translated_text.as_deref(), Some("xin chào"));
}
