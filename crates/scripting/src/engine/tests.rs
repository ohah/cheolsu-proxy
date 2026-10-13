use super::*;
use crate::types::{RequestAction, ResponseAction, ScriptRequest, ScriptResponse};
use std::collections::HashMap;

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

#[test]
fn test_no_hooks_returns_forward() {
    let mut engine = ScriptEngine::new().unwrap();
    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    assert!(matches!(result, RequestAction::Forward));
}

#[test]
fn test_on_request_forward() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(r#"cheolsu.onRequest((req) => ({ action: "forward" }))"#)
        .unwrap();
    assert!(engine.has_on_request());

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    assert!(matches!(result, RequestAction::Forward));
}

#[test]
fn test_on_request_respond() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onRequest((req) => {
                if (req.url.includes("blocked.com")) {
                    return {
                        action: "respond",
                        response: { status: 403, headers: {}, body: "Blocked" }
                    };
                }
                return { action: "forward" };
            });
        "#,
        )
        .unwrap();

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://blocked.com/page".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    let RequestAction::Respond { response } = result else {
        unreachable!("Respond 액션을 기대했지만 다른 값이 반환됨: {:?}", result);
    };
    assert_eq!(response.status, 403);
    assert_eq!(response.body.unwrap(), "Blocked");
}

#[test]
fn test_on_request_modify() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onRequest((req) => {
                req.headers["X-Custom"] = "injected";
                return { action: "modify", request: req };
            });
        "#,
        )
        .unwrap();

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://api.example.com/data".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    let RequestAction::ModifyRequest { request } = result else {
        unreachable!(
            "ModifyRequest 액션을 기대했지만 다른 값이 반환됨: {:?}",
            result
        );
    };
    assert_eq!(request.headers.get("X-Custom").unwrap(), "injected");
}

#[test]
fn test_on_response_modify() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onResponse((req, res) => {
                res.headers["X-Proxy"] = "cheolsu";
                return { action: "modify", response: res };
            });
        "#,
        )
        .unwrap();
    assert!(engine.has_on_response());

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let res = ScriptResponse {
        status: 200,
        headers: HashMap::new(),
        body: Some("OK".to_string()),
    };
    let result = block_on(engine.invoke_on_response(&req, &res)).unwrap();
    let ResponseAction::ModifyResponse { response } = result else {
        unreachable!(
            "ModifyResponse 액션을 기대했지만 다른 값이 반환됨: {:?}",
            result
        );
    };
    assert_eq!(response.headers.get("X-Proxy").unwrap(), "cheolsu");
}

#[test]
fn test_typescript_script() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_ts_code(
            r#"
            interface RequestContext {
                method: string;
                url: string;
                headers: Record<string, string>;
                body?: string;
            }
            cheolsu.onRequest((req: RequestContext) => {
                return { action: "forward" };
            });
        "#,
        )
        .unwrap();
    assert!(engine.has_on_request());
}

#[test]
fn test_error_in_hook_returns_forward() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onRequest((req) => {
                throw new Error("intentional error");
            });
        "#,
        )
        .unwrap();

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    assert!(matches!(result, RequestAction::Forward));
}

#[test]
fn test_special_chars_in_json() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(r#"cheolsu.onRequest((req) => ({ action: "forward" }))"#)
        .unwrap();

    let mut headers = HashMap::new();
    headers.insert(
        "X-Test".to_string(),
        "value with `backticks` and ${template}".to_string(),
    );
    let req = ScriptRequest {
        method: "GET".to_string(),
        url: r#"https://example.com/path?q=back\slash"#.to_string(),
        headers,
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    assert!(matches!(result, RequestAction::Forward));
}

#[test]
fn test_async_on_request() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onRequest(async (req) => {
                await new Promise(resolve => setTimeout(resolve, 10));
                req.headers["X-Async"] = "true";
                return { action: "modify", request: req };
            });
        "#,
        )
        .unwrap();
    assert!(engine.has_on_request());

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    let RequestAction::ModifyRequest { request } = result else {
        unreachable!(
            "비동기 훅에서 ModifyRequest 액션을 기대했지만 다른 값이 반환됨: {:?}",
            result
        );
    };
    assert_eq!(request.headers.get("X-Async").unwrap(), "true");
}

#[test]
fn test_async_on_response() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onResponse(async (req, res) => {
                await new Promise(resolve => setTimeout(resolve, 5));
                res.headers["X-Async"] = "response";
                return { action: "modify", response: res };
            });
        "#,
        )
        .unwrap();

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let res = ScriptResponse {
        status: 200,
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_response(&req, &res)).unwrap();
    let ResponseAction::ModifyResponse { response } = result else {
        unreachable!(
            "비동기 훅에서 ModifyResponse 액션을 기대했지만 다른 값이 반환됨: {:?}",
            result
        );
    };
    assert_eq!(response.headers.get("X-Async").unwrap(), "response");
}

#[test]
fn test_async_error_returns_forward() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onRequest(async (req) => {
                throw new Error("async error");
            });
        "#,
        )
        .unwrap();

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    assert!(matches!(result, RequestAction::Forward));
}

#[test]
fn test_clear_timers_prevents_pending_callbacks() {
    block_on(async {
        let mut engine = ScriptEngine::new().unwrap();
        engine
            .load_code(
                r#"
                let counter = 0;
                setInterval(() => { counter++; }, 50);
                setTimeout(() => { counter += 100; }, 50);
                cheolsu.onRequest((req) => {
                    return { action: "respond", response: { status: 200, headers: {}, body: String(counter) } };
                });
            "#,
            )
            .unwrap();

        // 타이머 정리
        engine.clear_timers();

        // 타이머 정리 후에도 엔진이 정상 동작해야 함
        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = engine.invoke_on_request(&req).await.unwrap();
        let RequestAction::Respond { response } = result else {
            unreachable!("Respond 액션을 기대했지만 다른 값이 반환됨: {:?}", result);
        };
        // counter가 0이어야 함 (타이머가 정리되어 콜백이 실행되지 않음)
        assert_eq!(response.body.unwrap(), "0");
    });
}

#[test]
fn test_reload_script_after_clear_timers() {
    block_on(async {
        let mut engine = ScriptEngine::new().unwrap();

        // 첫 번째 스크립트 로드: 타이머 사용
        engine
            .load_code(
                r#"
                setInterval(() => { console.log("tick"); }, 100);
                cheolsu.onRequest((req) => {
                    return { action: "respond", response: { status: 200, headers: {}, body: "v1" } };
                });
            "#,
            )
            .unwrap();
        assert!(engine.has_on_request());

        // 타이머 정리 후 안전하게 drop (실제 리로드 시나리오)
        engine.clear_timers();
        drop(engine);

        // 새 엔진 생성 후 다른 스크립트 로드
        let mut engine2 = ScriptEngine::new().unwrap();
        engine2
            .load_code(
                r#"
                cheolsu.onRequest((req) => {
                    return { action: "respond", response: { status: 200, headers: {}, body: "v2" } };
                });
            "#,
            )
            .unwrap();

        let req = ScriptRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = engine2.invoke_on_request(&req).await.unwrap();
        let RequestAction::Respond { response } = result else {
            unreachable!(
                "v2 Respond 액션을 기대했지만 다른 값이 반환됨: {:?}",
                result
            );
        };
        assert_eq!(response.body.unwrap(), "v2");
    });
}

#[test]
fn test_drain_logs() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            console.log("hello");
            console.error("oops");
            console.warn("careful");
        "#,
        )
        .unwrap();

    let logs = engine.drain_logs();
    assert_eq!(logs.len(), 3);
    assert_eq!(logs[0].level, "info");
    assert_eq!(logs[0].message, "hello");
    assert_eq!(logs[1].level, "error");
    assert_eq!(logs[1].message, "oops");
    assert_eq!(logs[2].level, "warn");
    assert_eq!(logs[2].message, "careful");

    // drain 후 비어있어야 함
    let logs2 = engine.drain_logs();
    assert!(logs2.is_empty());
}

#[test]
fn test_on_ws_message_forward() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(r#"cheolsu.onWebSocketMessage((msg) => ({ action: "forward" }))"#)
        .unwrap();
    assert!(engine.has_on_ws_message());
}

#[test]
fn test_set_timeout_basic() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            let called = false;
            cheolsu.onRequest(async (req) => {
                await new Promise(resolve => setTimeout(() => {
                    called = true;
                    resolve();
                }, 10));
                if (called) {
                    return { action: "respond", response: { status: 200, headers: {}, body: "timer works" } };
                }
                return { action: "forward" };
            });
        "#,
        )
        .unwrap();

    let req = ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    };
    let result = block_on(engine.invoke_on_request(&req)).unwrap();
    let RequestAction::Respond { response } = result else {
        unreachable!(
            "타이머 결과로 Respond 액션을 기대했지만 다른 값이 반환됨: {:?}",
            result
        );
    };
    assert_eq!(response.body.unwrap(), "timer works");
}

fn get_request() -> ScriptRequest {
    ScriptRequest {
        method: "GET".to_string(),
        url: "https://example.com".to_string(),
        headers: HashMap::new(),
        body: None,
    }
}

fn respond_body(result: RequestAction) -> String {
    let RequestAction::Respond { response } = result else {
        unreachable!("Respond 액션을 기대했지만 다른 값이 반환됨: {:?}", result);
    };
    response.body.unwrap_or_default()
}

/// V8 classic script와 같이 sloppy 모드로 실행되어야 한다 (QuickJS 기본값은 strict).
#[test]
fn test_scripts_run_in_sloppy_mode() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            undeclaredGlobal = "sloppy";
            cheolsu.onRequest(() => ({
                action: "respond",
                response: { status: 200, headers: {}, body: undeclaredGlobal }
            }));
        "#,
        )
        .unwrap();
    let result = block_on(engine.invoke_on_request(&get_request())).unwrap();
    assert_eq!(respond_body(result), "sloppy");
}

/// 엔진 내부 네이티브 함수와 이전 런타임 전역은 사용자 스크립트에 노출되지 않아야 한다.
#[test]
fn test_internal_natives_are_hidden() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onRequest(() => ({
                action: "respond",
                response: {
                    status: 200,
                    headers: {},
                    body: [typeof __cheolsu_now, typeof __cheolsu_print, typeof Deno].join(",")
                }
            }));
        "#,
        )
        .unwrap();
    let result = block_on(engine.invoke_on_request(&get_request())).unwrap();
    assert_eq!(respond_body(result), "undefined,undefined,undefined");
}

/// setInterval은 반복 실행되고, clearInterval 이후에는 멈춰야 한다.
/// 타이머는 만료 순서대로 실행되어야 한다.
#[test]
fn test_interval_and_timer_ordering() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            cheolsu.onRequest(async () => {
                const order = [];
                let ticks = 0;
                await new Promise((resolve) => {
                    const id = setInterval(() => {
                        ticks += 1;
                        if (ticks === 3) {
                            clearInterval(id);
                            resolve();
                        }
                    }, 10);
                });
                await new Promise((resolve) => {
                    setTimeout(() => order.push("b"), 20);
                    setTimeout(() => order.push("a"), 5);
                    setTimeout(() => { order.push("c"); resolve(); }, 30);
                });
                queueMicrotask(() => order.push("micro"));
                await null;
                return {
                    action: "respond",
                    response: { status: 200, headers: {}, body: ticks + ":" + order.join("") }
                };
            });
        "#,
        )
        .unwrap();
    let result = block_on(engine.invoke_on_request(&get_request())).unwrap();
    assert_eq!(respond_body(result), "3:abcmicro");
}

/// 해결되지 않는 Promise는 무한 대기 대신 오류로 끝나야 한다.
#[test]
fn test_never_resolving_promise_errors() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(r#"cheolsu.onRequest(() => new Promise(() => {}));"#)
        .unwrap();
    let result = block_on(engine.invoke_on_request(&get_request()));
    assert!(
        matches!(result, Err(ScriptError::Execution(_))),
        "해결되지 않는 Promise는 Execution 오류여야 함: {:?}",
        result
    );
}

/// 메모리 한도를 넘는 할당은 프로세스를 죽이지 않고 훅 오류(forward)로 처리되어야 하며,
/// 이후에도 엔진을 계속 사용할 수 있어야 한다.
#[test]
fn test_memory_limit_does_not_abort() {
    let mut engine = ScriptEngine::new().unwrap();
    engine
        .load_code(
            r#"
            let calls = 0;
            cheolsu.onRequest(() => {
                calls += 1;
                if (calls === 1) {
                    const hog = [];
                    while (true) hog.push(new Array(100000).fill(calls));
                }
                return { action: "respond", response: { status: 200, headers: {}, body: "alive" } };
            });
        "#,
        )
        .unwrap();
    let first = block_on(engine.invoke_on_request(&get_request()));
    assert!(
        matches!(
            first,
            Ok(RequestAction::Forward) | Err(ScriptError::Execution(_))
        ),
        "메모리 초과는 forward 또는 Execution 오류여야 함: {:?}",
        first
    );
    let second = block_on(engine.invoke_on_request(&get_request())).unwrap();
    assert_eq!(respond_body(second), "alive");
}

/// 문법 오류는 JS 오류 이름과 메시지를 담은 Execution 오류여야 한다.
#[test]
fn test_syntax_error_message() {
    let mut engine = ScriptEngine::new().unwrap();
    let err = engine.load_code("cheolsu.onRequest(() => {").unwrap_err();
    let ScriptError::Execution(message) = err else {
        unreachable!("Execution 오류를 기대함: {:?}", err);
    };
    assert!(message.contains("SyntaxError"), "메시지: {}", message);
}
