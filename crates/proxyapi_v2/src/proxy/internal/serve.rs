use super::InternalProxy;
use crate::{HttpHandler, WebSocketHandler, certificate_authority::CertificateAuthority};
use http::uri::{Authority, Scheme};
use hyper::{Request, Uri, service::service_fn};
use hyper_util::client::legacy::connect::Connect;
use tracing::{debug, error, warn};

impl<C, CA, H, W> InternalProxy<C, CA, H, W>
where
    C: Connect + Clone + Send + Sync + 'static,
    CA: CertificateAuthority,
    H: HttpHandler,
    W: WebSocketHandler,
{
    pub(crate) async fn serve_stream<I>(
        self,
        stream: I,
        scheme: Scheme,
        authority: Authority,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        I: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    {
        debug!(
            %authority,
            ?scheme,
            "[SERVE-STREAM] 스트림 서빙 시작"
        );

        let proxy_clone = self.clone();
        let service = service_fn({
            let authority = authority.clone();
            let scheme = scheme.clone();
            move |mut req| {
                debug!(
                    method = %req.method(),
                    uri = %req.uri(),
                    version = ?req.version(),
                    "[SERVE-STREAM] HTTP 요청 수신"
                );

                if req.version() == hyper::Version::HTTP_10
                    || req.version() == hyper::Version::HTTP_11
                {
                    let (mut parts, body) = req.into_parts();

                    parts.uri = {
                        let mut uri_parts = parts.uri.into_parts();
                        uri_parts.scheme = Some(scheme.clone());
                        uri_parts.authority = Some(authority.clone());
                        match Uri::from_parts(uri_parts) {
                            Ok(uri) => uri,
                            Err(e) => {
                                warn!("URI 재구성 실패: {}", e);
                                match Uri::builder()
                                    .scheme(scheme.clone())
                                    .authority(authority.clone())
                                    .path_and_query("/")
                                    .build()
                                {
                                    Ok(fallback) => fallback,
                                    Err(e2) => {
                                        error!("URI fallback 생성도 실패: {}", e2);
                                        Uri::builder()
                                            .path_and_query("/")
                                            .build()
                                            .unwrap_or_default()
                                    }
                                }
                            }
                        }
                    };

                    req = Request::from_parts(parts, body);
                    debug!(uri = %req.uri(), "[SERVE-STREAM] URI 재구성 완료");
                };

                debug!("[SERVE-STREAM] 프록시 요청 전달 시작");
                proxy_clone.clone().proxy(req)
            }
        });

        debug!("[SERVE-STREAM] 서버 연결 시작 - serve_connection_with_upgrades 호출");
        let result = self
            .server
            .serve_connection_with_upgrades(stream, service)
            .await;

        match result {
            Ok(_) => {
                debug!("[SERVE-STREAM] 스트림 서빙 완료: {}", authority);
                Ok(())
            }
            Err(e) => {
                // 에러 소스 체인을 모두 수집
                let mut error_chain = format!("{}", e);
                let mut source: Option<&dyn std::error::Error> = e.source();
                while let Some(s) = source {
                    error_chain.push_str(&format!(" → {}", s));
                    source = s.source();
                }

                let is_benign = error_chain.contains("error shutting down connection")
                    || error_chain.contains("close_notify")
                    || error_chain.contains("connection reset")
                    || error_chain.contains("broken pipe");

                if is_benign {
                    debug!(
                        %authority,
                        error_chain,
                        "[SERVE-STREAM] 연결 종료 (정상)"
                    );
                    Ok(())
                } else {
                    error!(
                        %authority,
                        error_chain,
                        "[SERVE-STREAM] 스트림 서빙 실패"
                    );
                    Err(e)
                }
            }
        }
    }
}
