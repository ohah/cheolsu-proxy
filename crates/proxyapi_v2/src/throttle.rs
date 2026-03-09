use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::Sleep;

/// 스로틀링 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottleConfig {
    /// 활성화 여부
    pub enabled: bool,
    /// 다운로드 속도 제한 (bytes/sec), None이면 무제한
    pub download_rate: Option<u64>,
    /// 업로드 속도 제한 (bytes/sec), None이면 무제한
    pub upload_rate: Option<u64>,
    /// 추가 지연 시간 (밀리초)
    pub latency_ms: u64,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            download_rate: None,
            upload_rate: None,
            latency_ms: 0,
        }
    }
}

/// 프리셋 프로필
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ThrottlePreset {
    /// No throttling
    None,
    /// GPRS: 50 KB/s down, 20 KB/s up, 500ms latency
    Gprs,
    /// Slow 3G: 500 KB/s down, 500 KB/s up, 400ms latency
    Slow3G,
    /// Fast 3G: 1.6 MB/s down, 768 KB/s up, 150ms latency
    Fast3G,
    /// 4G/LTE: 4 MB/s down, 3 MB/s up, 50ms latency
    Lte,
    /// WiFi: 30 MB/s down, 15 MB/s up, 2ms latency
    Wifi,
}

impl ThrottlePreset {
    pub fn to_config(self) -> ThrottleConfig {
        match self {
            Self::None => ThrottleConfig {
                enabled: false,
                ..Default::default()
            },
            Self::Gprs => ThrottleConfig {
                enabled: true,
                download_rate: Some(50 * 1024),
                upload_rate: Some(20 * 1024),
                latency_ms: 500,
            },
            Self::Slow3G => ThrottleConfig {
                enabled: true,
                download_rate: Some(500 * 1024),
                upload_rate: Some(500 * 1024),
                latency_ms: 400,
            },
            Self::Fast3G => ThrottleConfig {
                enabled: true,
                download_rate: Some(1_600 * 1024),
                upload_rate: Some(768 * 1024),
                latency_ms: 150,
            },
            Self::Lte => ThrottleConfig {
                enabled: true,
                download_rate: Some(4 * 1024 * 1024),
                upload_rate: Some(3 * 1024 * 1024),
                latency_ms: 50,
            },
            Self::Wifi => ThrottleConfig {
                enabled: true,
                download_rate: Some(30 * 1024 * 1024),
                upload_rate: Some(15 * 1024 * 1024),
                latency_ms: 2,
            },
        }
    }
}

/// Token Bucket 기반 속도 제한기
struct TokenBucket {
    tokens: f64,
    rate: f64, // bytes/sec
    last_refill: Instant,
}

impl TokenBucket {
    fn new(rate: u64) -> Self {
        Self {
            tokens: rate as f64, // 시작 시 1초분 토큰
            rate: rate as f64,
            last_refill: Instant::now(),
        }
    }

    /// 토큰 리필 후 사용 가능한 바이트 수 반환
    fn consume(&mut self, requested: usize) -> usize {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.last_refill = now;

        // 토큰 리필 (최대 1초분)
        self.tokens = (self.tokens + elapsed * self.rate).min(self.rate);

        if self.tokens < 1.0 {
            return 0;
        }

        let allowed = (self.tokens as usize).min(requested).max(1);
        self.tokens -= allowed as f64;
        allowed
    }

    /// 다음 토큰이 생길 때까지 대기할 시간
    fn time_until_available(&self) -> Duration {
        if self.tokens >= 1.0 {
            return Duration::ZERO;
        }
        let needed = 1.0 - self.tokens;
        Duration::from_secs_f64(needed / self.rate)
    }
}

/// 쓰로틀링이 적용된 양방향 복사.
/// client 쪽만 ThrottledIo로 감싸서 이중 제한을 방지한다.
/// - client → server: client의 read(download) 제한
/// - server → client: client의 write(upload) 제한
pub async fn copy_bidirectional_throttled<A, B>(
    client: &mut A,
    server: &mut B,
    config: &ThrottleConfig,
) -> std::io::Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let mut tc = ThrottledIo::new(client, config);
    tokio::io::copy_bidirectional(&mut tc, server).await
}

/// AsyncRead + AsyncWrite를 감싸서 속도를 제한하는 래퍼.
/// `copy_bidirectional`에서 사용: read = 다운로드, write = 업로드.
pub struct ThrottledIo<T> {
    inner: T,
    read_bucket: Option<TokenBucket>,
    write_bucket: Option<TokenBucket>,
    read_sleep: Option<Pin<Box<Sleep>>>,
    write_sleep: Option<Pin<Box<Sleep>>>,
    latency_applied: bool,
    latency: Duration,
    latency_sleep: Option<Pin<Box<Sleep>>>,
}

impl<T> ThrottledIo<T> {
    pub fn new(inner: T, config: &ThrottleConfig) -> Self {
        Self {
            inner,
            read_bucket: config.download_rate.map(TokenBucket::new),
            write_bucket: config.upload_rate.map(TokenBucket::new),
            read_sleep: None,
            write_sleep: None,
            latency_applied: false,
            latency: Duration::from_millis(config.latency_ms),
            latency_sleep: None,
        }
    }

    /// 스로틀링 없이 패스스루
    pub fn passthrough(inner: T) -> Self {
        Self {
            inner,
            read_bucket: None,
            write_bucket: None,
            read_sleep: None,
            write_sleep: None,
            latency_applied: true,
            latency: Duration::ZERO,
            latency_sleep: None,
        }
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for ThrottledIo<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();

        // 최초 latency 적용
        if !this.latency_applied {
            if this.latency > Duration::ZERO {
                let sleep = this
                    .latency_sleep
                    .get_or_insert_with(|| Box::pin(tokio::time::sleep(this.latency)));
                match sleep.as_mut().poll(cx) {
                    Poll::Ready(()) => {
                        this.latency_applied = true;
                        this.latency_sleep = None;
                    }
                    Poll::Pending => return Poll::Pending,
                }
            } else {
                this.latency_applied = true;
            }
        }

        // 속도 제한이 없으면 패스스루
        let bucket = match &mut this.read_bucket {
            Some(b) => b,
            None => return Pin::new(&mut this.inner).poll_read(cx, buf),
        };

        // sleep 중이면 대기
        if let Some(sleep) = &mut this.read_sleep {
            match sleep.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    this.read_sleep = None;
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        let allowed = bucket.consume(buf.remaining());
        if allowed == 0 {
            let wait = bucket.time_until_available();
            this.read_sleep = Some(Box::pin(tokio::time::sleep(wait)));
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        // 허용된 바이트만큼만 읽기
        let mut limited_buf = buf.take(allowed);
        match Pin::new(&mut this.inner).poll_read(cx, &mut limited_buf) {
            Poll::Ready(Ok(())) => {
                let filled = limited_buf.filled().len();
                // SAFETY: inner가 limited_buf에 쓴 만큼 원래 buf도 advance
                unsafe { buf.assume_init(filled) };
                buf.advance(filled);
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for ThrottledIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();

        // 속도 제한이 없으면 패스스루
        let bucket = match &mut this.write_bucket {
            Some(b) => b,
            None => return Pin::new(&mut this.inner).poll_write(cx, buf),
        };

        // sleep 중이면 대기
        if let Some(sleep) = &mut this.write_sleep {
            match sleep.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    this.write_sleep = None;
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        let allowed = bucket.consume(buf.len());
        if allowed == 0 {
            let wait = bucket.time_until_available();
            this.write_sleep = Some(Box::pin(tokio::time::sleep(wait)));
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        Pin::new(&mut this.inner).poll_write(cx, &buf[..allowed])
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn throttled_io_passthrough() {
        let data = b"hello world";
        let cursor = std::io::Cursor::new(data.to_vec());
        let mut throttled = ThrottledIo::passthrough(cursor);

        let mut buf = vec![0u8; 32];
        let n = throttled.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], data);
    }

    #[tokio::test]
    async fn throttled_io_limits_read_speed() {
        // 1KB/s 제한, 4KB 데이터 → 최소 3초 이상 걸려야 함
        let data = vec![0u8; 4096];
        let cursor = std::io::Cursor::new(data);
        let config = ThrottleConfig {
            enabled: true,
            download_rate: Some(1024),
            upload_rate: None,
            latency_ms: 0,
        };
        let mut throttled = ThrottledIo::new(cursor, &config);

        let start = Instant::now();
        let mut total = 0;
        let mut buf = vec![0u8; 256];
        loop {
            let n = throttled.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            total += n;
        }
        let elapsed = start.elapsed();

        assert_eq!(total, 4096);
        // 4KB / 1KB/s = ~4초이지만 시작 시 1초분 토큰이 있으므로 ~3초
        assert!(
            elapsed >= Duration::from_secs(2),
            "Should take at least 2s, took {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn throttle_config_default_is_disabled() {
        let config = ThrottleConfig::default();
        assert!(!config.enabled);
        assert!(config.download_rate.is_none());
        assert!(config.upload_rate.is_none());
        assert_eq!(config.latency_ms, 0);
    }

    #[tokio::test]
    async fn preset_slow_3g() {
        let config = ThrottlePreset::Slow3G.to_config();
        assert!(config.enabled);
        assert_eq!(config.download_rate, Some(500 * 1024));
        assert_eq!(config.upload_rate, Some(500 * 1024));
        assert_eq!(config.latency_ms, 400);
    }
}
