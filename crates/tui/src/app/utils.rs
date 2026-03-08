use proxy_v2_models::{RequestInfo, WsMessageInfo};

pub(crate) fn copy_to_clipboard(text: &str) -> bool {
    use std::process::{Command, Stdio};

    // macOS
    if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        return child.wait().map(|s| s.success()).unwrap_or(false);
    }

    // Linux (xclip)
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        return child.wait().map(|s| s.success()).unwrap_or(false);
    }

    false
}

pub(crate) fn format_ws_messages(conn_id: &str, uri: &str, messages: &[WsMessageInfo]) -> String {
    let mut out = format!("WebSocket: {}\n\n", uri);
    for msg in messages.iter().filter(|m| m.connection_id == conn_id) {
        let dir = match msg.direction {
            proxy_v2_models::WsDirection::ClientToServer => "->",
            proxy_v2_models::WsDirection::ServerToClient => "<-",
        };
        let msg_type = match msg.message_type {
            proxy_v2_models::WsMessageType::Text => "TXT",
            proxy_v2_models::WsMessageType::Binary => "BIN",
            proxy_v2_models::WsMessageType::Ping => "PING",
            proxy_v2_models::WsMessageType::Pong => "PONG",
            proxy_v2_models::WsMessageType::Close => "CLOSE",
        };
        out.push_str(&format!(
            "[{}] {} {} {}B\n{}\n\n",
            dir, msg_type, msg.size, msg.size, msg.payload
        ));
    }
    out
}

pub(crate) fn format_curl_command(info: &RequestInfo) -> String {
    let Some(req) = &info.0 else {
        return String::new();
    };

    let mut parts = vec![format!("curl -X {} '{}'", req.method(), req.uri())];

    for (name, value) in req.headers().iter() {
        let name_str = name.as_str();
        // host, content-length 등은 curl이 자동으로 설정
        if name_str == "host" || name_str == "content-length" {
            continue;
        }
        if let Ok(v) = value.to_str() {
            parts.push(format!("  -H '{}: {}'", name_str, v.replace('\'', "'\\''")));
        }
    }

    if let Some(body) = req.body() {
        if !body.is_empty() {
            if let Ok(body_str) = std::str::from_utf8(body) {
                parts.push(format!("  -d '{}'", body_str.replace('\'', "'\\''")));
            } else {
                parts.push("  --data-binary @-".to_string());
            }
        }
    }

    parts.join(" \\\n")
}
