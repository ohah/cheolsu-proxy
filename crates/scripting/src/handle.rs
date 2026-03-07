use crate::engine::ScriptEngine;
use crate::types::{
    RequestAction, ResponseAction, ScriptLogEntry, ScriptRequest, ScriptResponse, ScriptWsMessage,
    WsAction,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};

/// 스크립트 훅 실행 타임아웃 (30초)
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(30);

/// 스크립트 엔진으로 보내는 명령
enum ScriptCommand {
    LoadFile {
        path: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    LoadCode {
        code: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    LoadTsCode {
        code: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Unload {
        reply: oneshot::Sender<()>,
    },
    InvokeOnRequest {
        request: ScriptRequest,
        reply: oneshot::Sender<Result<RequestAction, String>>,
    },
    InvokeOnResponse {
        request: ScriptRequest,
        response: ScriptResponse,
        reply: oneshot::Sender<Result<ResponseAction, String>>,
    },
    InvokeOnWsMessage {
        message: ScriptWsMessage,
        reply: oneshot::Sender<Result<WsAction, String>>,
    },
    /// 엔진 스레드 종료
    Shutdown,
}

/// 스크립트 엔진에 대한 Send + Sync 핸들 (채널 기반)
#[derive(Clone)]
pub struct ScriptHandle {
    tx: mpsc::Sender<ScriptCommand>,
    active: Arc<std::sync::atomic::AtomicBool>,
    log_tx: broadcast::Sender<ScriptLogEntry>,
}

impl ScriptHandle {
    /// 새 스크립트 핸들 생성 (전용 스레드에서 엔진 실행)
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(128);
        let active = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (log_tx, _) = broadcast::channel(256);

        let active_clone = active.clone();
        let log_tx_clone = log_tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create scripting runtime");
            rt.block_on(script_engine_loop(rx, active_clone, log_tx_clone));
        });

        Self { tx, active, log_tx }
    }

    /// 엔진 스레드를 종료
    pub async fn shutdown(&self) {
        let _ = self.tx.send(ScriptCommand::Shutdown).await;
    }

    /// 스크립트가 활성화되어 있는지 확인
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Acquire)
    }

    /// 로그 수신 구독
    pub fn subscribe_logs(&self) -> broadcast::Receiver<ScriptLogEntry> {
        self.log_tx.subscribe()
    }

    /// 스크립트 파일 로드
    pub async fn load_file(&self, path: &str) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(ScriptCommand::LoadFile {
                path: path.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "스크립트 엔진 스레드 종료됨".to_string())?;
        tokio::time::timeout(SCRIPT_TIMEOUT, reply_rx)
            .await
            .map_err(|_| "스크립트 로드 타임아웃".to_string())?
            .map_err(|_| "스크립트 엔진 응답 없음".to_string())?
    }

    /// 스크립트 코드 로드 (JS)
    pub async fn load_code(&self, code: &str) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(ScriptCommand::LoadCode {
                code: code.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "스크립트 엔진 스레드 종료됨".to_string())?;
        tokio::time::timeout(SCRIPT_TIMEOUT, reply_rx)
            .await
            .map_err(|_| "스크립트 로드 타임아웃".to_string())?
            .map_err(|_| "스크립트 엔진 응답 없음".to_string())?
    }

    /// TypeScript 코드 로드
    pub async fn load_ts_code(&self, code: &str) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(ScriptCommand::LoadTsCode {
                code: code.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "스크립트 엔진 스레드 종료됨".to_string())?;
        tokio::time::timeout(SCRIPT_TIMEOUT, reply_rx)
            .await
            .map_err(|_| "스크립트 로드 타임아웃".to_string())?
            .map_err(|_| "스크립트 엔진 응답 없음".to_string())?
    }

    /// 스크립트 언로드
    pub async fn unload(&self) {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ScriptCommand::Unload { reply: reply_tx })
            .await;
        let _ = tokio::time::timeout(SCRIPT_TIMEOUT, reply_rx).await;
    }

    /// onRequest 훅 호출
    pub async fn invoke_on_request(
        &self,
        request: &ScriptRequest,
    ) -> Result<RequestAction, String> {
        if !self.is_active() {
            return Ok(RequestAction::Forward);
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(ScriptCommand::InvokeOnRequest {
                request: request.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "스크립트 엔진 스레드 종료됨".to_string())?;
        tokio::time::timeout(SCRIPT_TIMEOUT, reply_rx)
            .await
            .map_err(|_| "스크립트 실행 타임아웃 (onRequest)".to_string())?
            .map_err(|_| "스크립트 엔진 응답 없음".to_string())?
    }

    /// onResponse 훅 호출
    pub async fn invoke_on_response(
        &self,
        request: &ScriptRequest,
        response: &ScriptResponse,
    ) -> Result<ResponseAction, String> {
        if !self.is_active() {
            return Ok(ResponseAction::Forward);
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(ScriptCommand::InvokeOnResponse {
                request: request.clone(),
                response: response.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "스크립트 엔진 스레드 종료됨".to_string())?;
        tokio::time::timeout(SCRIPT_TIMEOUT, reply_rx)
            .await
            .map_err(|_| "스크립트 실행 타임아웃 (onResponse)".to_string())?
            .map_err(|_| "스크립트 엔진 응답 없음".to_string())?
    }

    /// onWebSocketMessage 훅 호출
    pub async fn invoke_on_ws_message(
        &self,
        message: &ScriptWsMessage,
    ) -> Result<WsAction, String> {
        if !self.is_active() {
            return Ok(WsAction::Forward);
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(ScriptCommand::InvokeOnWsMessage {
                message: message.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "스크립트 엔진 스레드 종료됨".to_string())?;
        tokio::time::timeout(SCRIPT_TIMEOUT, reply_rx)
            .await
            .map_err(|_| "스크립트 실행 타임아웃 (onWebSocketMessage)".to_string())?
            .map_err(|_| "스크립트 엔진 응답 없음".to_string())?
    }
}

/// 엔진에서 로그를 드레인하여 broadcast 채널로 전송
fn flush_logs(engine: &mut ScriptEngine, log_tx: &broadcast::Sender<ScriptLogEntry>) {
    for entry in engine.drain_logs() {
        let _ = log_tx.send(entry);
    }
}

/// 전용 스레드에서 실행되는 스크립트 엔진 이벤트 루프
async fn script_engine_loop(
    mut rx: mpsc::Receiver<ScriptCommand>,
    active: Arc<std::sync::atomic::AtomicBool>,
    log_tx: broadcast::Sender<ScriptLogEntry>,
) {
    let mut engine: Option<ScriptEngine> = None;

    while let Some(cmd) = rx.recv().await {
        match cmd {
            ScriptCommand::LoadFile { path, reply } => {
                let result = ScriptEngine::new().and_then(|mut e| {
                    e.load_script(&path)?;
                    flush_logs(&mut e, &log_tx);
                    Ok(e)
                });
                match result {
                    Ok(new_engine) => {
                        active.store(true, std::sync::atomic::Ordering::Release);
                        engine = Some(new_engine);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            ScriptCommand::LoadCode { code, reply } => {
                let result = ScriptEngine::new().and_then(|mut e| {
                    e.load_code(&code)?;
                    flush_logs(&mut e, &log_tx);
                    Ok(e)
                });
                match result {
                    Ok(new_engine) => {
                        active.store(true, std::sync::atomic::Ordering::Release);
                        engine = Some(new_engine);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            ScriptCommand::LoadTsCode { code, reply } => {
                let result = ScriptEngine::new().and_then(|mut e| {
                    e.load_ts_code(&code)?;
                    flush_logs(&mut e, &log_tx);
                    Ok(e)
                });
                match result {
                    Ok(new_engine) => {
                        active.store(true, std::sync::atomic::Ordering::Release);
                        engine = Some(new_engine);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            ScriptCommand::Unload { reply } => {
                engine = None;
                active.store(false, std::sync::atomic::Ordering::Release);
                let _ = reply.send(());
            }
            ScriptCommand::Shutdown => {
                engine = None;
                active.store(false, std::sync::atomic::Ordering::Release);
                break;
            }
            ScriptCommand::InvokeOnRequest { request, reply } => {
                let result = if let Some(e) = engine.as_mut() {
                    let r = e.invoke_on_request(&request);
                    flush_logs(e, &log_tx);
                    r
                } else {
                    Ok(RequestAction::Forward)
                };
                let _ = reply.send(result);
            }
            ScriptCommand::InvokeOnResponse {
                request,
                response,
                reply,
            } => {
                let result = if let Some(e) = engine.as_mut() {
                    let r = e.invoke_on_response(&request, &response);
                    flush_logs(e, &log_tx);
                    r
                } else {
                    Ok(ResponseAction::Forward)
                };
                let _ = reply.send(result);
            }
            ScriptCommand::InvokeOnWsMessage { message, reply } => {
                let result = if let Some(e) = engine.as_mut() {
                    let r = e.invoke_on_ws_message(&message);
                    flush_logs(e, &log_tx);
                    r
                } else {
                    Ok(WsAction::Forward)
                };
                let _ = reply.send(result);
            }
        }
    }
}
