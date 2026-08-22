use tokio_tungstenite::tungstenite::Message;

pub fn decode_ws_message(msg: Message) -> Option<String> {
    match msg {
        Message::Text(text) => Some(text.to_string()),
        Message::Binary(bytes) => String::from_utf8(bytes).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn decodes_binary_setup_complete_payload() {
        let payload = br#"{"setupComplete": {}}"#;
        let text = decode_ws_message(Message::Binary(payload.to_vec())).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert!(value.get("setupComplete").is_some());
    }
}
