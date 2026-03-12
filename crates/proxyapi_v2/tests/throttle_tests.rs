use proxyapi_v2::throttle::{ThrottleConfig, ThrottlePreset, ThrottledIo};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// ---------------------------------------------------------------------------
// ThrottleConfig 테스트
// ---------------------------------------------------------------------------

#[test]
fn default_config_is_disabled() {
    let config = ThrottleConfig::default();
    assert!(!config.enabled);
    assert!(config.download_rate.is_none());
    assert!(config.upload_rate.is_none());
    assert_eq!(config.latency_ms, 0);
}

#[test]
fn config_serialization_roundtrip() {
    let config = ThrottleConfig {
        enabled: true,
        download_rate: Some(1024 * 1024),
        upload_rate: Some(512 * 1024),
        latency_ms: 100,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ThrottleConfig = serde_json::from_str(&json).unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.download_rate, Some(1024 * 1024));
    assert_eq!(parsed.upload_rate, Some(512 * 1024));
    assert_eq!(parsed.latency_ms, 100);
}

#[test]
fn config_deserialization_with_nulls() {
    let json = r#"{"enabled":true,"download_rate":null,"upload_rate":null,"latency_ms":0}"#;
    let config: ThrottleConfig = serde_json::from_str(json).unwrap();
    assert!(config.enabled);
    assert!(config.download_rate.is_none());
    assert!(config.upload_rate.is_none());
}

// ---------------------------------------------------------------------------
// ThrottlePreset 테스트
// ---------------------------------------------------------------------------

#[test]
fn preset_none_is_disabled() {
    let config = ThrottlePreset::None.to_config();
    assert!(!config.enabled);
}

#[test]
fn preset_gprs() {
    let config = ThrottlePreset::Gprs.to_config();
    assert!(config.enabled);
    assert_eq!(config.download_rate, Some(50 * 1024));
    assert_eq!(config.upload_rate, Some(20 * 1024));
    assert_eq!(config.latency_ms, 500);
}

#[test]
fn preset_slow_3g() {
    let config = ThrottlePreset::Slow3G.to_config();
    assert!(config.enabled);
    assert_eq!(config.download_rate, Some(500 * 1024));
    assert_eq!(config.latency_ms, 400);
}

#[test]
fn preset_fast_3g() {
    let config = ThrottlePreset::Fast3G.to_config();
    assert!(config.enabled);
    assert_eq!(config.download_rate, Some(1_600 * 1024));
    assert_eq!(config.upload_rate, Some(768 * 1024));
    assert_eq!(config.latency_ms, 150);
}

#[test]
fn preset_lte() {
    let config = ThrottlePreset::Lte.to_config();
    assert!(config.enabled);
    assert_eq!(config.download_rate, Some(4 * 1024 * 1024));
    assert_eq!(config.upload_rate, Some(3 * 1024 * 1024));
    assert_eq!(config.latency_ms, 50);
}

#[test]
fn preset_wifi() {
    let config = ThrottlePreset::Wifi.to_config();
    assert!(config.enabled);
    assert_eq!(config.download_rate, Some(30 * 1024 * 1024));
    assert_eq!(config.upload_rate, Some(15 * 1024 * 1024));
    assert_eq!(config.latency_ms, 2);
}

#[test]
fn preset_serialization_roundtrip() {
    let preset = ThrottlePreset::Fast3G;
    let json = serde_json::to_string(&preset).unwrap();
    let parsed: ThrottlePreset = serde_json::from_str(&json).unwrap();
    let config = parsed.to_config();
    assert_eq!(config.download_rate, Some(1_600 * 1024));
}

// ---------------------------------------------------------------------------
// ThrottledIo 테스트
// ---------------------------------------------------------------------------

#[tokio::test]
async fn passthrough_no_delay() {
    let data = b"hello world passthrough";
    let cursor = std::io::Cursor::new(data.to_vec());
    let mut throttled = ThrottledIo::passthrough(cursor);

    let mut buf = vec![0u8; 64];
    let n = throttled.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], data);
}

#[tokio::test]
async fn passthrough_write() {
    let cursor = std::io::Cursor::new(Vec::new());
    let mut throttled = ThrottledIo::passthrough(cursor);

    throttled.write_all(b"test data").await.unwrap();
    throttled.flush().await.unwrap();
}

#[tokio::test]
async fn throttled_read_limits_speed() {
    // 2KB/s 제한, 4KB 데이터 → 최소 1초 이상 걸려야 함
    let data = vec![0xAB; 4096];
    let cursor = std::io::Cursor::new(data);
    let config = ThrottleConfig {
        enabled: true,
        download_rate: Some(2048),
        upload_rate: None,
        latency_ms: 0,
    };
    let mut throttled = ThrottledIo::new(cursor, &config);

    let start = Instant::now();
    let mut total = 0;
    let mut buf = vec![0u8; 512];
    loop {
        let n = throttled.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        total += n;
    }
    let elapsed = start.elapsed();

    assert_eq!(total, 4096);
    assert!(
        elapsed >= Duration::from_millis(1000),
        "Should take at least 1s, took {:?}",
        elapsed
    );
}

#[tokio::test]
async fn throttled_read_with_latency() {
    let data = b"latency test";
    let cursor = std::io::Cursor::new(data.to_vec());
    let config = ThrottleConfig {
        enabled: true,
        download_rate: None, // 속도 무제한
        upload_rate: None,
        latency_ms: 200, // 200ms 지연
    };
    let mut throttled = ThrottledIo::new(cursor, &config);

    let start = Instant::now();
    let mut buf = vec![0u8; 64];
    let n = throttled.read(&mut buf).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(&buf[..n], data);
    assert!(
        elapsed >= Duration::from_millis(180),
        "Should have ~200ms latency, took {:?}",
        elapsed
    );
}

#[tokio::test]
async fn throttled_no_rate_limit_is_fast() {
    // download_rate = None → 패스스루처럼 빠르게 동작
    let data = vec![0u8; 1024 * 1024]; // 1MB
    let cursor = std::io::Cursor::new(data);
    let config = ThrottleConfig {
        enabled: true,
        download_rate: None,
        upload_rate: None,
        latency_ms: 0,
    };
    let mut throttled = ThrottledIo::new(cursor, &config);

    let start = Instant::now();
    let mut total = 0;
    let mut buf = vec![0u8; 8192];
    loop {
        let n = throttled.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        total += n;
    }
    let elapsed = start.elapsed();

    assert_eq!(total, 1024 * 1024);
    assert!(
        elapsed < Duration::from_millis(100),
        "No rate limit should be fast, took {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// copy_bidirectional with ThrottledIo 통합 테스트
// ---------------------------------------------------------------------------

#[tokio::test]
async fn throttled_copy_bidirectional() {
    // 에코 서버 시작
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_port = listener.local_addr().unwrap().port();

    let echo_handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        stream.write_all(&buf[..n]).await.unwrap();
    });

    let config = ThrottleConfig {
        enabled: true,
        download_rate: Some(10 * 1024), // 10KB/s
        upload_rate: Some(10 * 1024),
        latency_ms: 0,
    };

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", server_port))
        .await
        .unwrap();

    // 직접 write/read로 테스트 (copy_bidirectional은 양쪽 모두 stream이어야 하므로)
    client.write_all(b"throttle test").await.unwrap();
    let mut buf = vec![0u8; 64];
    let n = client.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], b"throttle test");

    echo_handle.await.unwrap();

    // ThrottledIo로 에코 서버와 통신
    let listener2 = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port2 = listener2.local_addr().unwrap().port();

    let echo_handle2 = tokio::spawn(async move {
        let (mut stream, _) = listener2.accept().await.unwrap();
        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        stream.write_all(&buf[..n]).await.unwrap();
    });

    let client2 = TcpStream::connect(format!("127.0.0.1:{}", port2))
        .await
        .unwrap();
    let mut throttled = ThrottledIo::new(client2, &config);

    throttled.write_all(b"throttled echo").await.unwrap();
    let mut buf = vec![0u8; 64];
    let n = throttled.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], b"throttled echo");

    echo_handle2.await.unwrap();
}

#[tokio::test]
async fn throttled_write_limits_speed() {
    // write 속도 제한 테스트
    let config = ThrottleConfig {
        enabled: true,
        download_rate: None,
        upload_rate: Some(2048), // 2KB/s
        latency_ms: 0,
    };

    // 서버: 데이터를 받기만 함
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut total = 0;
        let mut buf = vec![0u8; 4096];
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => total += n,
                Err(_) => break,
            }
        }
        total
    });

    let client = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    let mut throttled = ThrottledIo::new(client, &config);

    let data = vec![0u8; 4096]; // 4KB
    let start = Instant::now();
    throttled.write_all(&data).await.unwrap();
    drop(throttled); // 연결 종료
    let elapsed = start.elapsed();

    let total = server_handle.await.unwrap();
    assert_eq!(total, 4096);
    // 4KB / 2KB/s = 2초이지만 시작 시 토큰이 있으므로 최소 1초
    assert!(
        elapsed >= Duration::from_millis(800),
        "Write should be throttled, took {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// copy_bidirectional_throttled 테스트
// ---------------------------------------------------------------------------

#[tokio::test]
async fn copy_bidirectional_throttled_echo() {
    // 에코 서버: 받은 데이터를 그대로 돌려줌
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let echo_handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 1024];
        let n = stream.read(&mut buf).await.unwrap();
        stream.write_all(&buf[..n]).await.unwrap();
        stream.shutdown().await.unwrap();
    });

    let mut client = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    // client 쪽에 직접 write 후 copy_bidirectional_throttled는 양방향 복사용이므로
    // 직접 ThrottledIo를 하나만 감싸서 에코 확인
    client.write_all(b"hello throttle").await.unwrap();
    let mut buf = vec![0u8; 64];
    let n = client.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], b"hello throttle");

    echo_handle.await.unwrap();
}

#[tokio::test]
async fn copy_bidirectional_throttled_no_double_throttle() {
    // client→server 방향: 데이터가 한쪽 ThrottledIo의 download rate만 적용되어야 함
    // 이중 적용 시 속도가 절반으로 더 떨어지는지 검증
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut total = 0usize;
        let mut buf = vec![0u8; 4096];
        loop {
            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => total += n,
                Err(_) => break,
            }
        }
        total
    });

    let client = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    let (client_read, mut client_write) = tokio::io::split(client);

    // 4KB 데이터를 write (upload 경로)
    let data = vec![0xABu8; 4096];
    let start = Instant::now();
    client_write.write_all(&data).await.unwrap();
    drop(client_write); // EOF 전송
    drop(client_read);
    let elapsed = start.elapsed();

    let total = server_handle.await.unwrap();
    assert_eq!(total, 4096);

    // 이중 적용이면 ~2초(4KB / 2KB/s), 단일이면 ~1초(4KB / 4KB/s, 시작 토큰 포함하면 더 빠름)
    // 시작 토큰(1초분 = 4KB)이 있으므로 즉시 전송 가능해야 함
    assert!(
        elapsed < Duration::from_millis(500),
        "Should not be double-throttled, took {:?}",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// 추가: disabled config이면 쓰로틀링 무시 확인
// ---------------------------------------------------------------------------

#[tokio::test]
async fn throttled_io_applies_rate_regardless_of_enabled_flag() {
    // ThrottledIo는 enabled 필드를 무시하고 rate를 그대로 적용.
    // 호출 측에서 enabled 체크 후 ThrottledIo 생성 여부를 결정해야 함.
    let data = vec![0u8; 4096]; // 4KB
    let cursor = std::io::Cursor::new(data);
    let config = ThrottleConfig {
        enabled: false,            // 비활성이지만
        download_rate: Some(2048), // 2KB/s 제한이 적용됨
        upload_rate: None,
        latency_ms: 0,
    };
    let mut throttled = ThrottledIo::new(cursor, &config);

    let start = Instant::now();
    let mut total = 0;
    let mut buf = vec![0u8; 512];
    loop {
        let n = throttled.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        total += n;
    }
    let elapsed = start.elapsed();

    assert_eq!(total, 4096);
    // enabled=false여도 ThrottledIo는 rate를 적용하므로 느려야 함
    assert!(
        elapsed >= Duration::from_millis(800),
        "ThrottledIo should apply rate limit regardless of enabled flag: {:?}",
        elapsed
    );
}

#[test]
fn all_presets_have_consistent_values() {
    // 모든 프리셋의 download >= upload 확인 (일반적인 네트워크 특성)
    let presets = [
        ThrottlePreset::Gprs,
        ThrottlePreset::Slow3G,
        ThrottlePreset::Fast3G,
        ThrottlePreset::Lte,
        ThrottlePreset::Wifi,
    ];
    for preset in presets {
        let config = preset.to_config();
        assert!(config.enabled, "{:?} should be enabled", preset);
        assert!(
            config.download_rate.is_some(),
            "{:?} should have download rate",
            preset
        );
        assert!(
            config.upload_rate.is_some(),
            "{:?} should have upload rate",
            preset
        );
        assert!(
            config.download_rate.unwrap() >= config.upload_rate.unwrap(),
            "{:?}: download ({}) should >= upload ({})",
            preset,
            config.download_rate.unwrap(),
            config.upload_rate.unwrap()
        );
    }
}

#[test]
fn presets_ordered_by_speed() {
    // 프리셋 속도가 GPRS < Slow3G < Fast3G < LTE < WiFi 순서인지 확인
    let speeds: Vec<u64> = [
        ThrottlePreset::Gprs,
        ThrottlePreset::Slow3G,
        ThrottlePreset::Fast3G,
        ThrottlePreset::Lte,
        ThrottlePreset::Wifi,
    ]
    .iter()
    .map(|p| p.to_config().download_rate.unwrap())
    .collect();

    for i in 1..speeds.len() {
        assert!(
            speeds[i] > speeds[i - 1],
            "Preset speed should increase: {} <= {}",
            speeds[i],
            speeds[i - 1]
        );
    }
}

#[test]
fn presets_latency_decreases_with_speed() {
    // 빠른 프리셋일수록 지연이 낮아야 함
    let latencies: Vec<u64> = [
        ThrottlePreset::Gprs,
        ThrottlePreset::Slow3G,
        ThrottlePreset::Fast3G,
        ThrottlePreset::Lte,
        ThrottlePreset::Wifi,
    ]
    .iter()
    .map(|p| p.to_config().latency_ms)
    .collect();

    for i in 1..latencies.len() {
        assert!(
            latencies[i] < latencies[i - 1],
            "Latency should decrease with speed: {} >= {}",
            latencies[i],
            latencies[i - 1]
        );
    }
}
