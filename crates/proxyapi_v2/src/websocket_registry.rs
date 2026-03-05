use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info};

/// 활성 WebSocket 연결에 메시지를 주입할 수 있는 핸들
#[derive(Clone)]
pub struct WebSocketInjector {
    /// 클라이언트 방향으로 메시지 주입 (서버→클라이언트처럼 보이게)
    to_client: mpsc::Sender<Message>,
    /// 서버 방향으로 메시지 주입 (클라이언트→서버처럼 보이게)
    to_server: mpsc::Sender<Message>,
}

impl WebSocketInjector {
    pub fn new(to_client: mpsc::Sender<Message>, to_server: mpsc::Sender<Message>) -> Self {
        Self {
            to_client,
            to_server,
        }
    }

    /// 클라이언트에게 메시지 주입 (서버가 보낸 것처럼)
    pub async fn inject_to_client(&self, msg: Message) -> Result<(), String> {
        self.to_client
            .send(msg)
            .await
            .map_err(|e| format!("클라이언트 주입 실패: {}", e))
    }

    /// 서버에게 메시지 주입 (클라이언트가 보낸 것처럼)
    pub async fn inject_to_server(&self, msg: Message) -> Result<(), String> {
        self.to_server
            .send(msg)
            .await
            .map_err(|e| format!("서버 주입 실패: {}", e))
    }
}

/// 활성 WebSocket 연결들을 관리하는 레지스트리
#[derive(Clone, Default)]
pub struct WebSocketRegistry {
    connections: Arc<RwLock<HashMap<String, WebSocketInjector>>>,
}

impl WebSocketRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 연결 등록
    pub async fn register(&self, id: String, injector: WebSocketInjector) {
        info!("[WS-REGISTRY] 연결 등록: {}", id);
        self.connections.write().await.insert(id, injector);
    }

    /// 연결 해제
    pub async fn unregister(&self, id: &str) {
        debug!("[WS-REGISTRY] 연결 해제: {}", id);
        self.connections.write().await.remove(id);
    }

    /// 특정 연결에 클라이언트 방향 메시지 주입
    pub async fn inject_to_client(&self, id: &str, msg: Message) -> Result<(), String> {
        let connections = self.connections.read().await;
        match connections.get(id) {
            Some(injector) => injector.inject_to_client(msg).await,
            None => Err(format!("연결 없음: {}", id)),
        }
    }

    /// 특정 연결에 서버 방향 메시지 주입
    pub async fn inject_to_server(&self, id: &str, msg: Message) -> Result<(), String> {
        let connections = self.connections.read().await;
        match connections.get(id) {
            Some(injector) => injector.inject_to_server(msg).await,
            None => Err(format!("연결 없음: {}", id)),
        }
    }

    /// 활성 연결 목록 조회
    pub async fn list_connections(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inject_to_client() {
        let (to_client_tx, mut to_client_rx) = mpsc::channel(16);
        let (to_server_tx, _to_server_rx) = mpsc::channel(16);

        let registry = WebSocketRegistry::new();
        let injector = WebSocketInjector::new(to_client_tx, to_server_tx);
        registry.register("test-conn".to_string(), injector).await;

        registry
            .inject_to_client("test-conn", Message::Text("hello".into()))
            .await
            .unwrap();

        let msg = to_client_rx.recv().await.unwrap();
        assert_eq!(msg, Message::Text("hello".into()));
    }

    #[tokio::test]
    async fn test_inject_to_server() {
        let (to_client_tx, _to_client_rx) = mpsc::channel(16);
        let (to_server_tx, mut to_server_rx) = mpsc::channel(16);

        let registry = WebSocketRegistry::new();
        let injector = WebSocketInjector::new(to_client_tx, to_server_tx);
        registry.register("test-conn".to_string(), injector).await;

        registry
            .inject_to_server("test-conn", Message::Text("from-proxy".into()))
            .await
            .unwrap();

        let msg = to_server_rx.recv().await.unwrap();
        assert_eq!(msg, Message::Text("from-proxy".into()));
    }

    #[tokio::test]
    async fn test_inject_nonexistent() {
        let registry = WebSocketRegistry::new();
        let result = registry
            .inject_to_client("missing", Message::Text("x".into()))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unregister() {
        let (to_client_tx, _) = mpsc::channel(16);
        let (to_server_tx, _) = mpsc::channel(16);

        let registry = WebSocketRegistry::new();
        registry
            .register(
                "conn1".to_string(),
                WebSocketInjector::new(to_client_tx, to_server_tx),
            )
            .await;

        assert_eq!(registry.list_connections().await.len(), 1);
        registry.unregister("conn1").await;
        assert_eq!(registry.list_connections().await.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        let registry = WebSocketRegistry::new();

        for i in 0..5 {
            let (tx_c, _) = mpsc::channel(16);
            let (tx_s, _) = mpsc::channel(16);
            registry
                .register(format!("conn-{}", i), WebSocketInjector::new(tx_c, tx_s))
                .await;
        }

        assert_eq!(registry.list_connections().await.len(), 5);

        registry.unregister("conn-2").await;
        assert_eq!(registry.list_connections().await.len(), 4);

        let result = registry
            .inject_to_client("conn-2", Message::Text("x".into()))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_register_overwrite() {
        let (tx_c1, _rx_c1) = mpsc::channel(16);
        let (tx_s1, _) = mpsc::channel(16);
        let (tx_c2, mut rx_c2) = mpsc::channel(16);
        let (tx_s2, _) = mpsc::channel(16);

        let registry = WebSocketRegistry::new();
        registry
            .register("conn".to_string(), WebSocketInjector::new(tx_c1, tx_s1))
            .await;
        registry
            .register("conn".to_string(), WebSocketInjector::new(tx_c2, tx_s2))
            .await;

        // 두 번째 등록이 덮어써야 함
        registry
            .inject_to_client("conn", Message::Text("test".into()))
            .await
            .unwrap();
        let msg = rx_c2.recv().await.unwrap();
        assert_eq!(msg, Message::Text("test".into()));
    }

    #[tokio::test]
    async fn test_inject_after_sender_dropped() {
        let (tx_c, rx_c) = mpsc::channel(16);
        let (tx_s, _) = mpsc::channel(16);

        let registry = WebSocketRegistry::new();
        registry
            .register("conn".to_string(), WebSocketInjector::new(tx_c, tx_s))
            .await;

        // receiver를 drop하면 sender가 실패해야 함
        drop(rx_c);

        let result = registry
            .inject_to_client("conn", Message::Text("x".into()))
            .await;
        assert!(result.is_err());
    }
}
