//! 스크립트 엔진 라이프사이클 통합 테스트
//!
//! ScriptHandle을 통한 스크립트 로드/언로드 및 훅 실행을 검증합니다.

use scripting::{
    RequestAction, ResponseAction, ScriptHandle, ScriptRequest, ScriptResponse, ScriptWsMessage,
    WsAction, WsDirection,
};
use std::collections::HashMap;

fn make_request(method: &str, url: &str) -> ScriptRequest {
    ScriptRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
        body: Some(r#"{"key":"value"}"#.to_string()),
    }
}

fn make_response(status: u16) -> ScriptResponse {
    ScriptResponse {
        status,
        headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
        body: Some(r#"{"result":"ok"}"#.to_string()),
    }
}

// ─── 기본 라이프사이클 ──────────────────────────────────────

#[tokio::test]
async fn new_handle_is_inactive() {
    let handle = ScriptHandle::new();
    assert!(!handle.is_active(), "새 핸들은 비활성 상태여야 함");
    handle.shutdown().await;
}

#[tokio::test]
async fn inactive_handle_returns_forward_for_request() {
    let handle = ScriptHandle::new();
    let req = make_request("GET", "https://example.com/api");
    let result = handle.invoke_on_request(&req).await;
    assert!(
        matches!(result, Ok(RequestAction::Forward)),
        "비활성 핸들은 Forward를 반환해야 함"
    );
    handle.shutdown().await;
}

#[tokio::test]
async fn inactive_handle_returns_forward_for_response() {
    let handle = ScriptHandle::new();
    let req = make_request("GET", "https://example.com/api");
    let res = make_response(200);
    let result = handle.invoke_on_response(&req, &res).await;
    assert!(
        matches!(result, Ok(ResponseAction::Forward)),
        "비활성 핸들은 Forward를 반환해야 함"
    );
    handle.shutdown().await;
}

#[tokio::test]
async fn inactive_handle_returns_forward_for_ws_message() {
    let handle = ScriptHandle::new();
    let msg = ScriptWsMessage {
        connection_id: "ws1".to_string(),
        url: "ws://example.com/ws".to_string(),
        direction: WsDirection::ToServer,
        payload: "hello".to_string(),
        is_binary: false,
    };
    let result = handle.invoke_on_ws_message(&msg).await;
    assert!(
        matches!(result, Ok(WsAction::Forward)),
        "비활성 핸들은 Forward를 반환해야 함"
    );
    handle.shutdown().await;
}

// ─── 스크립트 로드/언로드 ───────────────────────────────────

#[tokio::test]
async fn load_js_code_activates_handle() {
    let handle = ScriptHandle::new();
    let result = handle
        .load_code(r#"cheolsu.onRequest((req) => ({ action: "forward" }))"#)
        .await;
    assert!(result.is_ok(), "JS 코드 로드 성공해야 함");
    assert!(handle.is_active(), "코드 로드 후 활성 상태여야 함");
    handle.shutdown().await;
}

#[tokio::test]
async fn unload_deactivates_handle() {
    let handle = ScriptHandle::new();
    handle
        .load_code(r#"cheolsu.onRequest((req) => ({ action: "forward" }))"#)
        .await
        .unwrap();
    assert!(handle.is_active());

    handle.unload().await;
    assert!(!handle.is_active(), "언로드 후 비활성 상태여야 함");
    handle.shutdown().await;
}

#[tokio::test]
async fn reload_replaces_script() {
    let handle = ScriptHandle::new();

    // 첫 번째 스크립트: forward
    handle
        .load_code(r#"cheolsu.onRequest((req) => ({ action: "forward" }))"#)
        .await
        .unwrap();

    let req = make_request("GET", "https://example.com/api");
    let result = handle.invoke_on_request(&req).await.unwrap();
    assert!(matches!(result, RequestAction::Forward));

    // 두 번째 스크립트: respond
    handle
        .load_code(
            r#"cheolsu.onRequest((req) => ({
                action: "respond",
                response: { status: 403, headers: {}, body: "blocked" }
            }))"#,
        )
        .await
        .unwrap();

    let result = handle.invoke_on_request(&req).await.unwrap();
    assert!(
        matches!(result, RequestAction::Respond { .. }),
        "새 스크립트가 적용되어야 함"
    );
    handle.shutdown().await;
}

#[tokio::test]
async fn load_invalid_code_returns_error() {
    let handle = ScriptHandle::new();
    let result = handle.load_code("this is not valid javascript {{{").await;
    assert!(result.is_err(), "잘못된 코드는 에러를 반환해야 함");
    handle.shutdown().await;
}

#[tokio::test]
async fn load_nonexistent_file_returns_error() {
    let handle = ScriptHandle::new();
    let result = handle.load_file("/nonexistent/path/script.js").await;
    assert!(result.is_err(), "없는 파일은 에러를 반환해야 함");
    handle.shutdown().await;
}

// ─── 훅 실행 ────────────────────────────────────────────────

#[tokio::test]
async fn on_request_forward() {
    let handle = ScriptHandle::new();
    handle
        .load_code(r#"cheolsu.onRequest((req) => ({ action: "forward" }))"#)
        .await
        .unwrap();

    let req = make_request("GET", "https://example.com/api");
    let result = handle.invoke_on_request(&req).await.unwrap();
    assert!(matches!(result, RequestAction::Forward));
    handle.shutdown().await;
}

#[tokio::test]
async fn on_request_modify() {
    let handle = ScriptHandle::new();
    handle
        .load_code(
            r#"cheolsu.onRequest((req) => {
                req.headers["x-custom"] = "injected";
                return { action: "modify", request: req };
            })"#,
        )
        .await
        .unwrap();

    let req = make_request("POST", "https://api.example.com/data");
    let result = handle.invoke_on_request(&req).await.unwrap();
    match result {
        RequestAction::ModifyRequest { request } => {
            assert_eq!(request.headers.get("x-custom").unwrap(), "injected");
            assert_eq!(request.method, "POST");
        }
        _ => panic!("Expected ModifyRequest, got {:?}", result),
    }
    handle.shutdown().await;
}

#[tokio::test]
async fn on_request_respond_blocks_request() {
    let handle = ScriptHandle::new();
    handle
        .load_code(
            r#"cheolsu.onRequest((req) => {
                if (req.url.includes("blocked.com")) {
                    return {
                        action: "respond",
                        response: { status: 403, headers: {}, body: "Forbidden" }
                    };
                }
                return { action: "forward" };
            })"#,
        )
        .await
        .unwrap();

    // 차단되는 요청
    let req = make_request("GET", "https://blocked.com/page");
    let result = handle.invoke_on_request(&req).await.unwrap();
    match result {
        RequestAction::Respond { response } => {
            assert_eq!(response.status, 403);
        }
        _ => panic!("Expected Respond, got {:?}", result),
    }

    // 통과되는 요청
    let req = make_request("GET", "https://allowed.com/page");
    let result = handle.invoke_on_request(&req).await.unwrap();
    assert!(matches!(result, RequestAction::Forward));
    handle.shutdown().await;
}

#[tokio::test]
async fn on_response_modify() {
    let handle = ScriptHandle::new();
    handle
        .load_code(
            r#"cheolsu.onResponse((req, res) => {
                res.headers["x-proxy"] = "cheolsu";
                return { action: "modify", response: res };
            })"#,
        )
        .await
        .unwrap();

    let req = make_request("GET", "https://api.example.com/data");
    let res = make_response(200);
    let result = handle.invoke_on_response(&req, &res).await.unwrap();
    match result {
        ResponseAction::ModifyResponse { response } => {
            assert_eq!(response.headers.get("x-proxy").unwrap(), "cheolsu");
        }
        _ => panic!("Expected ModifyResponse, got {:?}", result),
    }
    handle.shutdown().await;
}

#[tokio::test]
async fn on_ws_message_modify() {
    let handle = ScriptHandle::new();
    handle
        .load_code(
            r#"cheolsu.onWebSocketMessage((msg) => ({
                action: "modify",
                payload: msg.payload + " [modified]",
                is_binary: false
            }))"#,
        )
        .await
        .unwrap();

    let msg = ScriptWsMessage {
        connection_id: "ws1".to_string(),
        url: "ws://example.com/ws".to_string(),
        direction: WsDirection::ToServer,
        payload: "hello".to_string(),
        is_binary: false,
    };
    let result = handle.invoke_on_ws_message(&msg).await.unwrap();
    match result {
        WsAction::Modify {
            payload, is_binary, ..
        } => {
            assert_eq!(payload, "hello [modified]");
            assert!(!is_binary);
        }
        _ => panic!("Expected Modify, got {:?}", result),
    }
    handle.shutdown().await;
}

#[tokio::test]
async fn on_ws_message_drop() {
    let handle = ScriptHandle::new();
    handle
        .load_code(
            r#"cheolsu.onWebSocketMessage((msg) => {
                if (msg.payload.includes("secret")) {
                    return { action: "drop" };
                }
                return { action: "forward" };
            })"#,
        )
        .await
        .unwrap();

    let msg = ScriptWsMessage {
        connection_id: "ws1".to_string(),
        url: "ws://example.com/ws".to_string(),
        direction: WsDirection::ToClient,
        payload: "secret data".to_string(),
        is_binary: false,
    };
    let result = handle.invoke_on_ws_message(&msg).await.unwrap();
    assert!(
        matches!(result, WsAction::Drop),
        "Expected Drop, got {:?}",
        result
    );

    let msg2 = ScriptWsMessage {
        connection_id: "ws1".to_string(),
        url: "ws://example.com/ws".to_string(),
        direction: WsDirection::ToClient,
        payload: "normal data".to_string(),
        is_binary: false,
    };
    let result = handle.invoke_on_ws_message(&msg2).await.unwrap();
    assert!(matches!(result, WsAction::Forward));
    handle.shutdown().await;
}

// ─── TypeScript 지원 ────────────────────────────────────────

#[tokio::test]
async fn load_typescript_code() {
    let handle = ScriptHandle::new();
    let result = handle
        .load_ts_code(
            r#"
            interface Request {
                method: string;
                url: string;
                headers: Record<string, string>;
                body?: string;
            }

            cheolsu.onRequest((req: Request) => {
                return { action: "forward" as const };
            });
            "#,
        )
        .await;
    assert!(
        result.is_ok(),
        "TypeScript 코드 로드 성공해야 함: {:?}",
        result.err()
    );
    assert!(handle.is_active());
    handle.shutdown().await;
}

// ─── 로그 구독 ──────────────────────────────────────────────

#[tokio::test]
async fn console_log_captured() {
    let handle = ScriptHandle::new();
    let mut log_rx = handle.subscribe_logs();

    handle
        .load_code(
            r#"
            console.log("hello from script");
            cheolsu.onRequest((req) => ({ action: "forward" }));
            "#,
        )
        .await
        .unwrap();

    // 로그 수신 대기 (최대 1초)
    let log = tokio::time::timeout(std::time::Duration::from_secs(1), log_rx.recv()).await;
    assert!(log.is_ok(), "로그를 수신해야 함");
    let entry = log.unwrap().unwrap();
    assert!(entry.message.contains("hello from script"));
    handle.shutdown().await;
}

// ─── 에러 복구 ──────────────────────────────────────────────

#[tokio::test]
async fn error_in_hook_does_not_panic() {
    let handle = ScriptHandle::new();
    handle
        .load_code(
            r#"cheolsu.onRequest((req) => {
                throw new Error("Intentional error");
            })"#,
        )
        .await
        .unwrap();

    let req = make_request("GET", "https://example.com/api");
    let result = handle.invoke_on_request(&req).await;
    // 에러 발생 시에도 panic하지 않아야 함
    assert!(result.is_ok() || result.is_err());
    handle.shutdown().await;
}
