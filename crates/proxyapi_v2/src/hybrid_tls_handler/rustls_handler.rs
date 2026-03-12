use crate::certificate_authority::CertificateAuthority;
use crate::rewind::Rewind;
use crate::tls_event::{TlsEvent, emit_tls_event};
use crate::upstream_cert::UpstreamCertInfo;
use http::uri::Authority;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_rustls::TlsAcceptor;
use tracing::{error, info};

use super::HybridTlsHandler;
use super::stream::HybridTlsStream;

impl<CA: CertificateAuthority> HybridTlsHandler<CA> {
    /// rustls를 사용하여 TLS 연결을 처리합니다
    #[allow(dead_code)]
    pub(super) async fn handle_with_rustls<R, W>(
        &self,
        authority: &Authority,
        stream: (R, W),
        initial_data: &[u8],
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (_read_stream, _write_stream) = stream;

        // 내부 버퍼를 사용하여 초기 데이터를 다시 읽을 수 있게 함
        let (client_read, client_write) = tokio::io::duplex(8192);

        // 초기 데이터를 내부 버퍼에 써넣기
        let mut client_write = client_write;
        client_write.write_all(initial_data).await?;
        client_write.flush().await?;
        drop(client_write);

        // Rewind 스트림 생성 - 초기 데이터를 먼저 읽을 수 있게 함
        let rewind_stream =
            Rewind::new(client_read, hyper::body::Bytes::from(initial_data.to_vec()));

        // 서버 설정 생성
        let server_config = self.ca.gen_server_config(authority, None).await?;
        let acceptor = TlsAcceptor::from(server_config);

        // TLS 핸드셰이크 수행
        match acceptor.accept(rewind_stream).await {
            Ok(tls_stream) => {
                info!("✅ rustls 핸드셰이크 성공: {}", authority);
                Ok(HybridTlsStream::RustlsGeneric(
                    tokio_rustls::TlsStream::Server(tls_stream),
                ))
            }
            Err(e) => {
                error!("❌ rustls 핸드셰이크 실패: {} - {}", authority, e);
                Err(format!("rustls handshake failed: {}", e).into())
            }
        }
    }

    /// rustls로 Upgraded 스트림을 처리합니다
    pub(super) async fn handle_with_rustls_upgraded(
        &self,
        authority: &Authority,
        upgraded: Rewind<TokioIo<Upgraded>>,
        _initial_buffer: &[u8],
        upstream_cert: Option<&UpstreamCertInfo>,
    ) -> Result<HybridTlsStream, Box<dyn std::error::Error + Send + Sync>> {
        emit_tls_event(
            &self.tls_event_sender,
            TlsEvent::FakeCertGenerating {
                authority: authority.clone(),
                has_upstream_cert: upstream_cert.is_some(),
            },
        );
        info!("🔧 [RUSTLS] 서버 설정 생성 시작: {}", authority);
        let server_config = self.ca.gen_server_config(authority, upstream_cert).await?;
        let acceptor = TlsAcceptor::from(server_config);
        info!("🔧 [RUSTLS] TlsAcceptor 생성 완료: {}", authority);

        info!("🔧 [RUSTLS] TLS 핸드셰이크 시작: {}", authority);
        let start_time = std::time::Instant::now();

        // rustls는 Rewind가 필요하므로 그대로 사용
        match acceptor.accept(upgraded).await {
            Ok(tls_stream) => {
                let duration = start_time.elapsed();
                info!(
                    "✅ [RUSTLS] 핸드셰이크 성공: {} (소요시간: {:?})",
                    authority, duration
                );

                // TLS 연결 정보 로깅
                if let Some(peer_cert) = tls_stream.get_ref().1.peer_certificates() {
                    info!("🔍 [RUSTLS] 피어 인증서 개수: {}", peer_cert.len());
                }

                Ok(HybridTlsStream::Rustls(tokio_rustls::TlsStream::Server(
                    tls_stream,
                )))
            }
            Err(e) => {
                let duration = start_time.elapsed();
                error!(
                    "❌ [RUSTLS] 핸드셰이크 실패: {} (소요시간: {:?})",
                    authority, duration
                );
                error!("❌ [RUSTLS] 오류 상세: {}", e);

                // 오류 타입별 상세 분석
                let error_str = e.to_string();
                if error_str.contains("eof") {
                    error!("🔍 [RUSTLS] EOF 오류 - 클라이언트가 연결을 끊었거나 예상치 못한 종료");
                } else if error_str.contains("alert") {
                    error!("🔍 [RUSTLS] TLS Alert 오류 - 프로토콜 위반 또는 보안 문제");
                } else if error_str.contains("certificate") {
                    error!("🔍 [RUSTLS] 인증서 관련 오류");
                } else if error_str.contains("cipher") {
                    error!("🔍 [RUSTLS] 암호화 스위트 관련 오류");
                } else {
                    error!("🔍 [RUSTLS] 기타 TLS 오류: {}", error_str);
                }

                Err(format!("rustls handshake failed: {}", e).into())
            }
        }
    }
}
