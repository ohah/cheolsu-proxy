use crate::engine::ScriptEngine;
use crate::error::ScriptError;
use crate::types::{
    RequestAction, ResponseAction, ScriptLogEntry, ScriptRequest, ScriptResponse, ScriptSseEvent,
    ScriptWsMessage, SseAction, WsAction,
};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};

/// 스크립트 훅 실행 타임아웃 (30초)
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(30);

/// 스크립트 엔진으로 보내는 명령
enum ScriptCommand {
    LoadFile {
        path: String,
        reply: oneshot::Sender<Result<(), ScriptError>>,
    },
    LoadCode {
        code: String,
        reply: oneshot::Sender<Result<(), ScriptError>>,
    },
    LoadTsCode {
        code: String,
        reply: oneshot::Sender<Result<(), ScriptError>>,
    },
    Unload {
        reply: oneshot::Sender<()>,
    },
    InvokeOnRequest {
        request: ScriptRequest,
        reply: oneshot::Sender<Result<RequestAction, ScriptError>>,
    },
    InvokeOnResponse {
        request: ScriptRequest,
        response: ScriptResponse,
        reply: oneshot::Sender<Result<ResponseAction, ScriptError>>,
    },
    InvokeOnWsMessage {
        message: ScriptWsMessage,
        reply: oneshot::Sender<Result<WsAction, ScriptError>>,
    },
    InvokeOnSseEvent {
        event: ScriptSseEvent,
        reply: oneshot::Sender<Result<SseAction, ScriptError>>,
    },
    /// 엔진 스레드 종료
    Shutdown,
}

/// 스크립트 커맨드 채널 용량
/// 스크립트 실행 요청을 큐잉하는 mpsc 채널의 버퍼 크기.
/// 동시 다발적인 요청 처리 시에도 백프레셔 없이 수용할 수 있는 충분한 크기.
const SCRIPT_COMMAND_CHANNEL_CAPACITY: usize = 128;

/// 스크립트 로그 브로드캐스트 채널 용량
/// 여러 구독자에게 스크립트 로그를 전송하는 broadcast 채널의 버퍼 크기.
/// 빠른 로그 출력 시에도 구독자 lag를 최소화하기 위한 크기.
const SCRIPT_LOG_CHANNEL_CAPACITY: usize = 256;

/// 엔진 스레드 외부에서 실행 중인 스크립트를 강제 종료하기 위한 제어 채널.
/// `isolate`는 현재 엔진의 종료 핸들, `terminated`는 강제 종료가 요청되었음을 알리는 플래그다.
struct EngineControl {
    isolate: Mutex<Option<v8::IsolateHandle>>,
    terminated: std::sync::atomic::AtomicBool,
}

/// 스크립트 엔진에 대한 Send + Sync 핸들 (채널 기반)
#[derive(Clone)]
pub struct ScriptHandle {
    tx: Option<mpsc::Sender<ScriptCommand>>,
    active: Arc<std::sync::atomic::AtomicBool>,
    loaded_path: Arc<std::sync::RwLock<Option<String>>>,
    log_tx: broadcast::Sender<ScriptLogEntry>,
    thread_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// 훅/로드 응답 대기 타임아웃 (기본 SCRIPT_TIMEOUT, 테스트에서 조정 가능)
    timeout: Duration,
    control: Arc<EngineControl>,
}

impl ScriptHandle {
    /// 새 스크립트 핸들 생성 (전용 스레드에서 엔진 실행)
    pub fn new() -> Self {
        Self::new_with_timeout(SCRIPT_TIMEOUT)
    }

    /// 타임아웃을 지정하여 핸들 생성 (테스트/특수 용도)
    pub fn new_with_timeout(timeout: Duration) -> Self {
        let (tx, rx) = mpsc::channel(SCRIPT_COMMAND_CHANNEL_CAPACITY);
        let active = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (log_tx, _) = broadcast::channel(SCRIPT_LOG_CHANNEL_CAPACITY);
        let control = Arc::new(EngineControl {
            isolate: Mutex::new(None),
            terminated: std::sync::atomic::AtomicBool::new(false),
        });

        let active_clone = active.clone();
        let log_tx_clone = log_tx.clone();
        let control_clone = control.clone();
        let handle = std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create scripting runtime: {}", e);
                    return;
                }
            };
            rt.block_on(script_engine_loop(
                rx,
                active_clone,
                log_tx_clone,
                control_clone,
            ));
        });

        Self {
            tx: Some(tx),
            active,
            loaded_path: Arc::new(std::sync::RwLock::new(None)),
            log_tx,
            thread_handle: Arc::new(Mutex::new(Some(handle))),
            timeout,
            control,
        }
    }

    /// 응답 타임아웃 시 호출: 엔진 스레드에서 실행 중인(무한 루프 등) 스크립트를
    /// V8 terminate_execution으로 강제 중단시켜 엔진 스레드가 다음 명령을 처리할 수 있게 한다.
    fn terminate_running_script(&self) {
        self.control
            .terminated
            .store(true, std::sync::atomic::Ordering::Release);
        if let Ok(guard) = self.control.isolate.lock() {
            if let Some(h) = guard.as_ref() {
                h.terminate_execution();
            }
        }
    }

    /// 엔진 스레드를 종료하고 join하여 리소스 회수
    pub async fn shutdown(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(ScriptCommand::Shutdown).await;
        }
        // 엔진 스레드가 무한 루프 등으로 멈춰 있으면 Shutdown 명령을 처리하지 못해 join이
        // 영구 블로킹된다. V8 실행을 강제 종료해 엔진 스레드가 명령 루프로 복귀하도록 한다.
        self.terminate_running_script();
        // 엔진 스레드가 종료될 때까지 대기하여 리소스 leak 방지
        let handle = self.thread_handle.lock().ok().and_then(|mut g| g.take());
        if let Some(h) = handle {
            let _ = tokio::task::spawn_blocking(move || h.join()).await;
        }
    }

    /// 내부 전송 채널 참조 반환
    fn sender(&self) -> Result<&mpsc::Sender<ScriptCommand>, ScriptError> {
        self.tx.as_ref().ok_or(ScriptError::EngineShutdown)
    }

    /// 스크립트가 활성화되어 있는지 확인
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Acquire)
    }

    /// 현재 로드된 스크립트 경로 반환
    pub fn loaded_path(&self) -> Option<String> {
        self.loaded_path
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 로그 수신 구독
    pub fn subscribe_logs(&self) -> broadcast::Receiver<ScriptLogEntry> {
        self.log_tx.subscribe()
    }

    /// 스크립트 파일 로드
    pub async fn load_file(&self, path: &str) -> Result<(), ScriptError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender()?
            .send(ScriptCommand::LoadFile {
                path: path.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ScriptError::EngineShutdown)?;
        let result = match tokio::time::timeout(self.timeout, reply_rx).await {
            Ok(inner) => inner.map_err(|_| ScriptError::NoResponse)?,
            Err(_) => {
                self.terminate_running_script();
                return Err(ScriptError::Timeout("스크립트 로드".to_string()));
            }
        };
        if result.is_ok() {
            *self.loaded_path.write().unwrap_or_else(|e| e.into_inner()) = Some(path.to_string());
        }
        result
    }

    /// 스크립트 코드 로드 (JS)
    pub async fn load_code(&self, code: &str) -> Result<(), ScriptError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender()?
            .send(ScriptCommand::LoadCode {
                code: code.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ScriptError::EngineShutdown)?;
        let result = match tokio::time::timeout(self.timeout, reply_rx).await {
            Ok(inner) => inner.map_err(|_| ScriptError::NoResponse)?,
            Err(_) => {
                self.terminate_running_script();
                return Err(ScriptError::Timeout("스크립트 로드".to_string()));
            }
        };
        if result.is_ok() {
            *self.loaded_path.write().unwrap_or_else(|e| e.into_inner()) = None;
        }
        result
    }

    /// TypeScript 코드 로드
    pub async fn load_ts_code(&self, code: &str) -> Result<(), ScriptError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender()?
            .send(ScriptCommand::LoadTsCode {
                code: code.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ScriptError::EngineShutdown)?;
        let result = match tokio::time::timeout(self.timeout, reply_rx).await {
            Ok(inner) => inner.map_err(|_| ScriptError::NoResponse)?,
            Err(_) => {
                self.terminate_running_script();
                return Err(ScriptError::Timeout("스크립트 로드".to_string()));
            }
        };
        if result.is_ok() {
            *self.loaded_path.write().unwrap_or_else(|e| e.into_inner()) = None;
        }
        result
    }

    /// 스크립트 언로드
    pub async fn unload(&self) {
        let (reply_tx, reply_rx) = oneshot::channel();
        if let Some(tx) = &self.tx {
            let _ = tx.send(ScriptCommand::Unload { reply: reply_tx }).await;
        }
        if tokio::time::timeout(self.timeout, reply_rx).await.is_err() {
            self.terminate_running_script();
        }
        *self.loaded_path.write().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// onRequest 훅 호출
    pub async fn invoke_on_request(
        &self,
        request: &ScriptRequest,
    ) -> Result<RequestAction, ScriptError> {
        if !self.is_active() {
            return Ok(RequestAction::Forward);
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender()?
            .send(ScriptCommand::InvokeOnRequest {
                request: request.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ScriptError::EngineShutdown)?;
        match tokio::time::timeout(self.timeout, reply_rx).await {
            Ok(inner) => inner.map_err(|_| ScriptError::NoResponse)?,
            Err(_) => {
                self.terminate_running_script();
                Err(ScriptError::Timeout("onRequest".to_string()))
            }
        }
    }

    /// onResponse 훅 호출
    pub async fn invoke_on_response(
        &self,
        request: &ScriptRequest,
        response: &ScriptResponse,
    ) -> Result<ResponseAction, ScriptError> {
        if !self.is_active() {
            return Ok(ResponseAction::Forward);
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender()?
            .send(ScriptCommand::InvokeOnResponse {
                request: request.clone(),
                response: response.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ScriptError::EngineShutdown)?;
        match tokio::time::timeout(self.timeout, reply_rx).await {
            Ok(inner) => inner.map_err(|_| ScriptError::NoResponse)?,
            Err(_) => {
                self.terminate_running_script();
                Err(ScriptError::Timeout("onResponse".to_string()))
            }
        }
    }

    /// onWebSocketMessage 훅 호출
    pub async fn invoke_on_ws_message(
        &self,
        message: &ScriptWsMessage,
    ) -> Result<WsAction, ScriptError> {
        if !self.is_active() {
            return Ok(WsAction::Forward);
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender()?
            .send(ScriptCommand::InvokeOnWsMessage {
                message: message.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ScriptError::EngineShutdown)?;
        match tokio::time::timeout(self.timeout, reply_rx).await {
            Ok(inner) => inner.map_err(|_| ScriptError::NoResponse)?,
            Err(_) => {
                self.terminate_running_script();
                Err(ScriptError::Timeout("onWebSocketMessage".to_string()))
            }
        }
    }

    /// onSSEMessage 훅 호출
    pub async fn invoke_on_sse_event(
        &self,
        event: &ScriptSseEvent,
    ) -> Result<SseAction, ScriptError> {
        if !self.is_active() {
            return Ok(SseAction::Forward);
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        self.sender()?
            .send(ScriptCommand::InvokeOnSseEvent {
                event: event.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| ScriptError::EngineShutdown)?;
        match tokio::time::timeout(self.timeout, reply_rx).await {
            Ok(inner) => inner.map_err(|_| ScriptError::NoResponse)?,
            Err(_) => {
                self.terminate_running_script();
                Err(ScriptError::Timeout("onSSEMessage".to_string()))
            }
        }
    }
}

impl Drop for ScriptHandle {
    fn drop(&mut self) {
        // Arc의 마지막 소유자인 경우에만 스레드를 정리
        if Arc::strong_count(&self.thread_handle) == 1 {
            if let Some(tx) = self.tx.take() {
                let handle = self.thread_handle.clone();
                // 별도 스레드에서 정리하여 tokio 런타임 스레드에서의 데드락 방지
                std::thread::spawn(move || {
                    let _ = tx.blocking_send(ScriptCommand::Shutdown);
                    if let Ok(mut guard) = handle.lock() {
                        if let Some(h) = guard.take() {
                            let _ = h.join();
                        }
                    }
                });
            }
        }
    }
}

/// 엔진에서 로그를 드레인하여 broadcast 채널로 전송
fn flush_logs(engine: &mut ScriptEngine, log_tx: &broadcast::Sender<ScriptLogEntry>) {
    for entry in engine.drain_logs() {
        let _ = log_tx.send(entry);
    }
}

/// 이전 엔진의 타이머를 정리하고 안전하게 drop (V8 crash 방지)
fn safely_drop_engine(engine: &mut Option<ScriptEngine>) {
    if let Some(old) = engine.as_mut() {
        old.clear_timers();
    }
    *engine = None;
}

/// 엔진을 교체하고 결과를 reply 채널로 전송
fn replace_engine(
    engine: &mut Option<ScriptEngine>,
    active: &std::sync::atomic::AtomicBool,
    log_tx: &broadcast::Sender<ScriptLogEntry>,
    result: Result<ScriptEngine, ScriptError>,
    reply: oneshot::Sender<Result<(), ScriptError>>,
) {
    match result {
        Ok(new_engine) => {
            active.store(true, std::sync::atomic::Ordering::Release);
            *engine = Some(new_engine);
            let _ = reply.send(Ok(()));
        }
        Err(e) => {
            active.store(false, std::sync::atomic::Ordering::Release);
            let _ = reply.send(Err(e));
        }
    }
    // 새 엔진 로드 직후 잔여 로그 flush
    if let Some(e) = engine.as_mut() {
        flush_logs(e, log_tx);
    }
}

/// 엔진 제어의 isolate 핸들 슬롯을 갱신한다.
fn set_isolate(control: &Arc<EngineControl>, handle: Option<v8::IsolateHandle>) {
    if let Ok(mut g) = control.isolate.lock() {
        *g = handle;
    }
}

/// 기존 엔진을 정리하고 isolate 핸들 슬롯도 비운다.
fn drop_engine_and_clear(engine: &mut Option<ScriptEngine>, control: &Arc<EngineControl>) {
    safely_drop_engine(engine);
    set_isolate(control, None);
}

/// 새 엔진을 만들고(핸들을 먼저 등록해 로드 중 무한 루프도 종료 가능) 로더를 실행한다.
fn build_engine<F>(
    control: &Arc<EngineControl>,
    log_tx: &broadcast::Sender<ScriptLogEntry>,
    load: F,
) -> Result<ScriptEngine, ScriptError>
where
    F: FnOnce(&mut ScriptEngine) -> Result<(), ScriptError>,
{
    let mut e = ScriptEngine::new()?;
    // 로드(최상위 코드)에서 무한 루프가 발생해도 외부에서 terminate할 수 있도록 핸들을 먼저 등록
    set_isolate(control, Some(e.isolate_handle()));
    let r = load(&mut e);
    flush_logs(&mut e, log_tx);
    if control
        .terminated
        .swap(false, std::sync::atomic::Ordering::AcqRel)
    {
        e.cancel_termination();
        set_isolate(control, None);
        return Err(ScriptError::Timeout(
            "스크립트 로드 중 시간 초과로 강제 종료됨".to_string(),
        ));
    }
    match r {
        Ok(()) => Ok(e),
        Err(err) => {
            set_isolate(control, None);
            Err(err)
        }
    }
}

/// 강제 종료가 요청된 경우 엔진을 폐기하여 이후 호출이 다시 멈추지 않도록 복구한다.
fn recover_if_terminated(
    engine: &mut Option<ScriptEngine>,
    active: &std::sync::atomic::AtomicBool,
    control: &Arc<EngineControl>,
) {
    if control
        .terminated
        .swap(false, std::sync::atomic::Ordering::AcqRel)
    {
        if let Some(e) = engine.as_mut() {
            e.cancel_termination();
        }
        drop_engine_and_clear(engine, control);
        active.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// 전용 스레드에서 실행되는 스크립트 엔진 이벤트 루프
async fn script_engine_loop(
    mut rx: mpsc::Receiver<ScriptCommand>,
    active: Arc<std::sync::atomic::AtomicBool>,
    log_tx: broadcast::Sender<ScriptLogEntry>,
    control: Arc<EngineControl>,
) {
    let mut engine: Option<ScriptEngine> = None;

    while let Some(cmd) = rx.recv().await {
        match cmd {
            ScriptCommand::LoadFile { path, reply } => {
                drop_engine_and_clear(&mut engine, &control);
                let result = build_engine(&control, &log_tx, |e| e.load_script(&path));
                replace_engine(&mut engine, &active, &log_tx, result, reply);
            }
            ScriptCommand::LoadCode { code, reply } => {
                drop_engine_and_clear(&mut engine, &control);
                let result = build_engine(&control, &log_tx, |e| e.load_code(&code));
                replace_engine(&mut engine, &active, &log_tx, result, reply);
            }
            ScriptCommand::LoadTsCode { code, reply } => {
                drop_engine_and_clear(&mut engine, &control);
                let result = build_engine(&control, &log_tx, |e| e.load_ts_code(&code));
                replace_engine(&mut engine, &active, &log_tx, result, reply);
            }
            ScriptCommand::Unload { reply } => {
                drop_engine_and_clear(&mut engine, &control);
                active.store(false, std::sync::atomic::Ordering::Release);
                let _ = reply.send(());
            }
            ScriptCommand::Shutdown => {
                drop_engine_and_clear(&mut engine, &control);
                active.store(false, std::sync::atomic::Ordering::Release);
                break;
            }
            ScriptCommand::InvokeOnRequest { request, reply } => {
                let result = if let Some(e) = engine.as_mut() {
                    let r = e.invoke_on_request(&request).await;
                    flush_logs(e, &log_tx);
                    r
                } else {
                    Ok(RequestAction::Forward)
                };
                recover_if_terminated(&mut engine, &active, &control);
                let _ = reply.send(result);
            }
            ScriptCommand::InvokeOnResponse {
                request,
                response,
                reply,
            } => {
                let result = if let Some(e) = engine.as_mut() {
                    let r = e.invoke_on_response(&request, &response).await;
                    flush_logs(e, &log_tx);
                    r
                } else {
                    Ok(ResponseAction::Forward)
                };
                recover_if_terminated(&mut engine, &active, &control);
                let _ = reply.send(result);
            }
            ScriptCommand::InvokeOnWsMessage { message, reply } => {
                let result = if let Some(e) = engine.as_mut() {
                    let r = e.invoke_on_ws_message(&message).await;
                    flush_logs(e, &log_tx);
                    r
                } else {
                    Ok(WsAction::Forward)
                };
                recover_if_terminated(&mut engine, &active, &control);
                let _ = reply.send(result);
            }
            ScriptCommand::InvokeOnSseEvent { event, reply } => {
                let result = if let Some(e) = engine.as_mut() {
                    let r = e.invoke_on_sse_event(&event).await;
                    flush_logs(e, &log_tx);
                    r
                } else {
                    Ok(SseAction::Forward)
                };
                recover_if_terminated(&mut engine, &active, &control);
                let _ = reply.send(result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn dummy_request() -> ScriptRequest {
        ScriptRequest {
            method: "GET".to_string(),
            url: "http://example.com/".to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    /// 무한 루프 훅이 타임아웃으로 강제 종료되고, 엔진 스레드가 복구되어
    /// 이후 정상 스크립트 로드가 동작하는지 검증한다(영구 행/DoS 방지).
    #[tokio::test]
    async fn infinite_loop_hook_terminates_and_engine_recovers() {
        let handle = ScriptHandle::new_with_timeout(Duration::from_millis(800));

        // onRequest에 무한 루프를 등록(등록 자체는 즉시 완료)
        handle
            .load_code("cheolsu.onRequest((req) => { while (true) {} });")
            .await
            .expect("무한 루프 훅 로드는 성공해야 함");

        // 훅 호출은 타임아웃되어야 하며 내부적으로 V8 실행을 강제 종료한다
        let r = handle.invoke_on_request(&dummy_request()).await;
        assert!(
            matches!(r, Err(ScriptError::Timeout(_))),
            "무한 루프 훅은 Timeout이어야 함, got {:?}",
            r
        );

        // 엔진 스레드가 복구되어 새 정상 스크립트 로드가 정상 동작해야 함(영구 행 방지)
        handle
            .load_code("cheolsu.onRequest((req) => ({ action: 'forward' }));")
            .await
            .expect("강제 종료 후 엔진이 복구되어 재로드가 성공해야 함");

        handle.shutdown().await;
    }

    /// 엔진이 무한 루프로 멈춰 있어도 shutdown이 무한 블로킹되지 않고 완료되어야 한다.
    #[tokio::test]
    async fn shutdown_completes_even_when_engine_stuck() {
        // 타임아웃을 길게 둬서 invoke가 자체 타임아웃으로 끝나지 않도록 한다.
        let handle = ScriptHandle::new_with_timeout(Duration::from_secs(30));
        handle
            .load_code("cheolsu.onRequest((req) => { while (true) {} });")
            .await
            .expect("훅 로드 성공");

        // 무한 루프 훅을 백그라운드로 호출하여 엔진 스레드를 멈춘다.
        let h2 = handle.clone();
        let invoke = tokio::spawn(async move {
            let _ = h2.invoke_on_request(&dummy_request()).await;
        });
        tokio::time::sleep(Duration::from_millis(300)).await;

        // 엔진이 멈춘 상태에서도 shutdown이 합리적 시간 안에 완료되어야 한다.
        tokio::time::timeout(Duration::from_secs(5), handle.shutdown())
            .await
            .expect("shutdown이 무한 블로킹 없이 완료되어야 함");

        invoke.abort();
    }
}
