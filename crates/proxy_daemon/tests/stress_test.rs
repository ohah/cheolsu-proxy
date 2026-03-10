//! 스트레스/부하 테스트
//!
//! 프록시 데몬의 동시성, 대용량 처리, 빠른 연결/해제 등 극한 상황을 시뮬레이션합니다.
//! `#[ignore]` 어트리뷰트가 적용되어 있으므로 일반 CI에서는 실행되지 않습니다.
//!
//! 수동 실행: `cargo test -p proxy_daemon --test stress_test -- --ignored --nocapture`

use proxy_daemon::{
    DaemonMessage, InterceptAction, InterceptRule, LoggingHandler, ServerReplayEntry,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// 테스트용 LoggingHandler 생성 헬퍼
///
/// 각 호출마다 고유한 임시 디렉토리를 생성하여 테스트 간 캐시 경로 충돌을 방지합니다.
fn make_handler() -> (LoggingHandler, tempfile::TempDir) {
    let tmp_dir = tempfile::tempdir().expect("임시 디렉토리 생성 실패");
    let (sender, _rx) = tokio::sync::mpsc::channel(16);
    let handler = LoggingHandler::new(sender, tmp_dir.path().to_path_buf());
    (handler, tmp_dir)
}

/// 테스트용 InterceptRule 생성 헬퍼
fn make_block_rule(id: &str, pattern: &str) -> InterceptRule {
    InterceptRule {
        id: id.to_string(),
        name: format!("Stress Rule {}", id),
        enabled: true,
        pattern: pattern.to_string(),
        method: None,
        action: InterceptAction::Block {
            status_code: 403,
            body: format!("Blocked by {}", id),
        },
    }
}

// ─── 다수 동시 클라이언트 연결 시뮬레이션 ───────────────────────

/// 100개의 동시 클라이언트가 핸들러에 인터셉트 규칙을 업데이트하는 시나리오
#[tokio::test]
#[ignore]
async fn stress_concurrent_intercept_rule_updates() {
    let (handler, _tmp_dir) = make_handler();
    let handler = Arc::new(handler);
    let mut handles = Vec::new();

    // 100개의 동시 태스크에서 인터셉트 규칙 업데이트
    for i in 0..100 {
        let handler = handler.clone();
        handles.push(tokio::spawn(async move {
            let rules = vec![
                make_block_rule(&format!("r{}-1", i), &format!("*client{}.com*", i)),
                make_block_rule(&format!("r{}-2", i), &format!("*api{}.com*", i)),
            ];
            handler.update_intercept_rules(rules).await;
        }));
    }

    // 모든 태스크가 패닉 없이 완료되는지 확인
    for handle in handles {
        handle
            .await
            .expect("동시 인터셉트 규칙 업데이트 태스크가 패닉");
    }
}

/// 200개의 동시 클라이언트가 서버 리플레이 엔트리를 업데이트하는 시나리오
#[tokio::test]
#[ignore]
async fn stress_concurrent_server_replay_updates() {
    let (handler, _tmp_dir) = make_handler();
    let handler = Arc::new(handler);
    let mut handles = Vec::new();

    for i in 0..200 {
        let handler = handler.clone();
        handles.push(tokio::spawn(async move {
            let entries = vec![ServerReplayEntry {
                id: format!("replay-{}", i),
                method: "GET".to_string(),
                url: format!("https://api{}.example.com/data", i),
                status: 200,
                headers: HashMap::from([(
                    "content-type".to_string(),
                    "application/json".to_string(),
                )]),
                body: Some(format!(r#"{{"client":{}}}"#, i)),
            }];
            handler.update_server_replay_entries(entries).await;
        }));
    }

    for handle in handles {
        handle
            .await
            .expect("동시 서버 리플레이 업데이트 태스크가 패닉");
    }
}

/// 인터셉트 규칙과 서버 리플레이를 동시에 업데이트하는 혼합 시나리오
#[tokio::test]
#[ignore]
async fn stress_mixed_concurrent_updates() {
    let (handler, _tmp_dir) = make_handler();
    let handler = Arc::new(handler);
    let mut handles = Vec::new();

    for i in 0..150 {
        let handler = handler.clone();
        if i % 2 == 0 {
            // 짝수: 인터셉트 규칙 업데이트
            handles.push(tokio::spawn(async move {
                let rules = vec![make_block_rule(
                    &format!("mixed-{}", i),
                    &format!("*mixed{}.com*", i),
                )];
                handler.update_intercept_rules(rules).await;
            }));
        } else {
            // 홀수: 서버 리플레이 업데이트
            handles.push(tokio::spawn(async move {
                let entries = vec![ServerReplayEntry {
                    id: format!("mixed-replay-{}", i),
                    method: "POST".to_string(),
                    url: format!("https://mixed{}.example.com/api", i),
                    status: 201,
                    headers: HashMap::new(),
                    body: None,
                }];
                handler.update_server_replay_entries(entries).await;
            }));
        }
    }

    for handle in handles {
        handle.await.expect("혼합 동시 업데이트 태스크가 패닉");
    }
}

// ─── 대용량 요청/응답 바디 처리 ─────────────────────────────────

/// 10MB 이상의 대용량 바디를 가진 ServerReplayEntry 직렬화/역직렬화 테스트
#[tokio::test]
#[ignore]
async fn stress_large_body_serialization() {
    // 10MB 문자열 생성
    let large_body = "A".repeat(10 * 1024 * 1024);
    let entry = ServerReplayEntry {
        id: "large-body-1".to_string(),
        method: "GET".to_string(),
        url: "https://api.example.com/large-data".to_string(),
        status: 200,
        headers: HashMap::from([
            (
                "content-type".to_string(),
                "application/octet-stream".to_string(),
            ),
            ("content-length".to_string(), large_body.len().to_string()),
        ]),
        body: Some(large_body.clone()),
    };

    // 대용량 바디 직렬화가 패닉 없이 성공하는지 확인
    let json = serde_json::to_string(&entry).expect("10MB 바디 직렬화 실패");
    let parsed: ServerReplayEntry = serde_json::from_str(&json).expect("10MB 바디 역직렬화 실패");
    assert_eq!(
        parsed.body.as_ref().map(|b| b.len()),
        Some(large_body.len())
    );
    assert_eq!(parsed.id, "large-body-1");
}

/// 대용량 바디를 핸들러에 서버 리플레이로 등록하는 테스트
#[tokio::test]
#[ignore]
async fn stress_large_body_server_replay_update() {
    let (handler, _tmp_dir) = make_handler();

    // 15MB 바디
    let large_body = "B".repeat(15 * 1024 * 1024);
    let entries = vec![ServerReplayEntry {
        id: "large-replay-1".to_string(),
        method: "POST".to_string(),
        url: "https://api.example.com/upload".to_string(),
        status: 200,
        headers: HashMap::new(),
        body: Some(large_body),
    }];

    // 패닉 없이 업데이트 성공
    handler.update_server_replay_entries(entries).await;
}

/// 다수의 대용량 엔트리를 동시에 처리하는 테스트
#[tokio::test]
#[ignore]
async fn stress_multiple_large_bodies_concurrent() {
    let (handler, _tmp_dir) = make_handler();
    let handler = Arc::new(handler);
    let mut handles = Vec::new();

    // 10개의 태스크에서 각각 1MB 바디를 가진 엔트리를 등록
    for i in 0..10 {
        let handler = handler.clone();
        handles.push(tokio::spawn(async move {
            let body = "C".repeat(1024 * 1024); // 1MB
            let entries = vec![ServerReplayEntry {
                id: format!("concurrent-large-{}", i),
                method: "GET".to_string(),
                url: format!("https://api{}.example.com/data", i),
                status: 200,
                headers: HashMap::new(),
                body: Some(body),
            }];
            handler.update_server_replay_entries(entries).await;
        }));
    }

    for handle in handles {
        handle
            .await
            .expect("동시 대용량 바디 업데이트 태스크가 패닉");
    }
}

/// 대용량 헤더 맵을 포함한 인터셉트 규칙 처리 테스트
#[tokio::test]
#[ignore]
async fn stress_large_headers_intercept_rule() {
    let (handler, _tmp_dir) = make_handler();

    // 1000개의 커스텀 헤더를 포함하는 ModifyRequest 규칙
    let mut headers = HashMap::new();
    for i in 0..1000 {
        headers.insert(format!("X-Custom-Header-{}", i), format!("value-{}", i));
    }

    let rules = vec![InterceptRule {
        id: "large-headers".to_string(),
        name: "Large Headers Rule".to_string(),
        enabled: true,
        pattern: "*api.example.com*".to_string(),
        method: None,
        action: InterceptAction::ModifyRequest {
            add_headers: headers,
            remove_headers: (0..500).map(|i| format!("X-Remove-{}", i)).collect(),
            set_body: None,
        },
    }];

    // 패닉 없이 업데이트 성공
    handler.update_intercept_rules(rules).await;
}

// ─── 빠른 연결/해제 반복 (Rapid Connect/Disconnect) ─────────────

/// 빠르게 반복적으로 핸들러를 생성하고 파괴하는 테스트
#[tokio::test]
#[ignore]
async fn stress_rapid_handler_create_destroy() {
    for i in 0..500 {
        let tmp_dir = tempfile::tempdir().expect("임시 디렉토리 생성 실패");
        let (sender, _rx) = tokio::sync::mpsc::channel(16);
        let handler = LoggingHandler::new(sender, tmp_dir.path().to_path_buf());

        // 규칙 업데이트 후 즉시 Drop
        let rules = vec![make_block_rule(&format!("rapid-{}", i), "*rapid*")];
        handler.update_intercept_rules(rules).await;
        // handler와 tmp_dir는 여기서 Drop됨
    }
}

/// mpsc 채널의 빠른 생성/해제 반복 테스트
#[tokio::test]
#[ignore]
async fn stress_rapid_channel_create_destroy() {
    for _ in 0..1000 {
        let (sender, mut rx) = tokio::sync::mpsc::channel::<String>(64);

        // 즉시 메시지 전송 후 수신
        let _ = sender.send("test".to_string()).await;
        let received = rx.recv().await;
        assert!(received.is_some());

        // sender와 rx가 여기서 Drop됨
    }
}

/// 동시에 핸들러 생성/파괴를 반복하는 테스트
#[tokio::test]
#[ignore]
async fn stress_concurrent_rapid_handler_lifecycle() {
    let mut handles = Vec::new();

    for batch in 0..50 {
        handles.push(tokio::spawn(async move {
            for i in 0..20 {
                let tmp_dir = tempfile::tempdir().expect("임시 디렉토리 생성 실패");
                let (sender, _rx) = tokio::sync::mpsc::channel(16);
                let handler = LoggingHandler::new(sender, tmp_dir.path().to_path_buf());
                let rules = vec![make_block_rule(&format!("rapid-{}-{}", batch, i), "*test*")];
                handler.update_intercept_rules(rules).await;
            }
        }));
    }

    for handle in handles {
        handle
            .await
            .expect("동시 핸들러 생명주기 반복 태스크가 패닉");
    }
}

/// AtomicUsize 기반 클라이언트 카운트의 빠른 증감 반복 테스트
#[tokio::test]
#[ignore]
async fn stress_rapid_atomic_count_increment_decrement() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    // 500개의 동시 태스크에서 카운트 증가 후 감소
    for _ in 0..500 {
        let count = count.clone();
        handles.push(tokio::spawn(async move {
            count.fetch_add(1, Ordering::SeqCst);
            // 약간의 작업 시뮬레이션
            tokio::task::yield_now().await;
            count.fetch_sub(1, Ordering::SeqCst);
        }));
    }

    for handle in handles {
        handle.await.expect("원자적 카운트 증감 태스크가 패닉");
    }

    // 모든 태스크 완료 후 카운트는 0이어야 함
    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "모든 클라이언트 연결/해제 후 카운트는 0이어야 함"
    );
}

// ─── Broadcast 채널 대량 메시지 전송 ────────────────────────────

/// broadcast 채널에 대량 메시지를 전송하고 수신하는 테스트
#[tokio::test]
#[ignore]
async fn stress_broadcast_high_volume_messages() {
    let (tx, _) = broadcast::channel::<String>(4096);
    let mut handles = Vec::new();
    let message_count = Arc::new(AtomicUsize::new(0));
    let total_messages: usize = 1000;

    // 10개의 수신자 생성
    for _ in 0..10 {
        let mut rx = tx.subscribe();
        let count = message_count.clone();
        handles.push(tokio::spawn(async move {
            let mut received = 0;
            while let Ok(_msg) = rx.recv().await {
                received += 1;
                count.fetch_add(1, Ordering::Relaxed);
                if received >= total_messages {
                    break;
                }
            }
        }));
    }

    // 대량 메시지 전송
    for i in 0..total_messages {
        let msg = DaemonMessage::Status {
            running: true,
            port: 8100,
        };
        let json = serde_json::to_string(&msg).expect("DaemonMessage 직렬화 실패");
        tx.send(format!("{}:{}", i, json))
            .expect("broadcast 메시지 전송 실패");
    }

    // 모든 수신자 태스크 완료 대기
    for handle in handles {
        handle.await.expect("broadcast 수신 태스크가 패닉");
    }

    // 10개 수신자 × 1000개 메시지 = 10000개
    let total_received = message_count.load(Ordering::Relaxed);
    assert_eq!(
        total_received,
        10 * total_messages,
        "모든 수신자가 모든 메시지를 수신해야 함: 기대={}, 실제={}",
        10 * total_messages,
        total_received
    );
}

/// broadcast 채널에서 수신자가 느릴 때 lagged 에러 처리 테스트
#[tokio::test]
#[ignore]
async fn stress_broadcast_lagged_receivers() {
    // 작은 버퍼로 broadcast 채널 생성 (lagged 발생 유도)
    let (tx, _) = broadcast::channel::<String>(32);
    let mut handles = Vec::new();
    let lagged_count = Arc::new(AtomicUsize::new(0));

    // 느린 수신자 5개 생성
    for _ in 0..5 {
        let mut rx = tx.subscribe();
        let lagged = lagged_count.clone();
        handles.push(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // 뒤처진 메시지 수 기록
                        lagged.fetch_add(n as usize, Ordering::Relaxed);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
    }

    // 버퍼보다 많은 메시지를 빠르게 전송하여 lagged 유도
    for i in 0..500 {
        let _ = tx.send(format!("message-{}", i));
    }

    // 송신자 Drop → 수신자에게 Closed 전달
    drop(tx);

    for handle in handles {
        handle.await.expect("lagged 수신자 태스크가 패닉");
    }

    // lagged가 발생했어도 패닉 없이 정상 처리되었는지 확인
    let total_lagged = lagged_count.load(Ordering::Relaxed);
    // 작은 버퍼(32)에 500개 전송했으므로 lagged 발생이 예상됨
    assert!(
        total_lagged > 0,
        "작은 버퍼에 대량 전송 시 lagged가 발생해야 함"
    );
}

/// 다수 수신자가 동시에 구독/해제를 반복하는 테스트
#[tokio::test]
#[ignore]
async fn stress_broadcast_rapid_subscribe_unsubscribe() {
    let (tx, _) = broadcast::channel::<String>(256);
    let tx = Arc::new(tx);
    let mut handles = Vec::new();

    // 50개의 태스크에서 구독/수신/해제 반복
    for _ in 0..50 {
        let tx = tx.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..100 {
                let mut rx = tx.subscribe();
                // 채널에 메시지가 있으면 수신 시도 (논블로킹)
                let _ = rx.try_recv();
                // rx Drop → 구독 해제
            }
        }));
    }

    // 동시에 메시지 전송도 수행
    let tx_sender = tx.clone();
    let sender_handle = tokio::spawn(async move {
        for i in 0..500 {
            let _ = tx_sender.send(format!("sub-unsub-{}", i));
            tokio::task::yield_now().await;
        }
    });

    for handle in handles {
        handle.await.expect("구독/해제 반복 태스크가 패닉");
    }
    sender_handle.await.expect("메시지 전송 태스크가 패닉");
}

/// broadcast 채널을 통한 대량 DaemonMessage 직렬화 전송 테스트
#[tokio::test]
#[ignore]
async fn stress_broadcast_daemon_messages_serialization() {
    let (tx, _) = broadcast::channel::<String>(2048);
    let received_count = Arc::new(AtomicUsize::new(0));

    // 수신자 생성
    let mut rx = tx.subscribe();
    let count = received_count.clone();
    let receiver_handle = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // 역직렬화 검증
            let _parsed: DaemonMessage =
                serde_json::from_str(&msg).expect("수신된 DaemonMessage 역직렬화 실패");
            count.fetch_add(1, Ordering::Relaxed);
        }
    });

    // 다양한 DaemonMessage 타입을 대량 전송
    let messages: Vec<DaemonMessage> = vec![
        DaemonMessage::Status {
            running: true,
            port: 8100,
        },
        DaemonMessage::ScriptResult {
            success: true,
            error: None,
        },
        DaemonMessage::ScriptLog {
            level: "info".to_string(),
            message: "test log".to_string(),
        },
        DaemonMessage::ScriptStatus {
            active: true,
            path: Some("/tmp/test.ts".to_string()),
            message: "loaded".to_string(),
        },
        DaemonMessage::WsInjectResult {
            success: false,
            error: Some("not found".to_string()),
        },
    ];

    let total_sent = 2000;
    for i in 0..total_sent {
        let msg = &messages[i % messages.len()];
        let json = serde_json::to_string(msg).expect("DaemonMessage 직렬화 실패");
        tx.send(json).expect("broadcast 전송 실패");
    }

    // 송신자 Drop
    drop(tx);
    receiver_handle.await.expect("수신자 태스크가 패닉");

    assert_eq!(
        received_count.load(Ordering::Relaxed),
        total_sent,
        "전송된 모든 메시지가 정상적으로 역직렬화되어야 함"
    );
}

// ─── 복합 스트레스 시나리오 ─────────────────────────────────────

/// 대량 인터셉트 규칙을 한 번에 등록하는 테스트
#[tokio::test]
#[ignore]
async fn stress_bulk_intercept_rules() {
    let (handler, _tmp_dir) = make_handler();

    // 10000개의 인터셉트 규칙을 한 번에 등록
    let rules: Vec<InterceptRule> = (0..10000)
        .map(|i| make_block_rule(&format!("bulk-{}", i), &format!("*host{}.example.com*", i)))
        .collect();

    handler.update_intercept_rules(rules).await;
}

/// watch 채널의 빠른 값 갱신 테스트
#[tokio::test]
#[ignore]
async fn stress_watch_channel_rapid_updates() {
    let (tx, mut rx) = tokio::sync::watch::channel::<Vec<InterceptRule>>(Vec::new());
    let tx = Arc::new(tx);
    let update_count = Arc::new(AtomicUsize::new(0));

    // 수신자: 변경 감지
    let count = update_count.clone();
    let receiver_handle = tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let _rules = rx.borrow().clone();
            count.fetch_add(1, Ordering::Relaxed);
        }
    });

    // 송신자: 빠른 연속 갱신
    for i in 0..1000 {
        let rules = vec![make_block_rule(&format!("watch-{}", i), "*test*")];
        tx.send(rules).expect("watch 채널 전송 실패");
    }

    // 송신자 Drop
    drop(tx);
    receiver_handle.await.expect("watch 수신자 태스크가 패닉");

    // watch 채널은 최신 값만 보관하므로 수신 횟수가 전송 횟수보다 작을 수 있음
    let received = update_count.load(Ordering::Relaxed);
    assert!(
        received > 0,
        "watch 채널에서 최소 1회 이상 변경이 감지되어야 함"
    );
}

/// DaemonMessage 직렬화 대량 반복 테스트 (메모리 누수 확인용)
#[tokio::test]
#[ignore]
async fn stress_serialization_deserialization_loop() {
    let messages: Vec<DaemonMessage> = vec![
        DaemonMessage::Status {
            running: true,
            port: 8100,
        },
        DaemonMessage::ScriptResult {
            success: true,
            error: None,
        },
        DaemonMessage::ScriptLog {
            level: "warn".to_string(),
            message: "stress test warning".to_string(),
        },
    ];

    // 100,000회 직렬화/역직렬화 반복
    for i in 0..100_000 {
        let msg = &messages[i % messages.len()];
        let json = serde_json::to_string(msg).expect("직렬화 실패");
        let _parsed: DaemonMessage = serde_json::from_str(&json).expect("역직렬화 실패");
    }
}
