//! .proto 파일 기반 gRPC 메시지 디코딩 레지스트리
//!
//! 사용자가 .proto 파일을 등록하면, 캡처된 gRPC 트래픽의
//! 필드 번호 대신 실제 필드명과 타입을 표시할 수 있습니다.

use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Proto 파일 레지스트리: .proto 파일을 파싱하고 gRPC 메시지를 디코딩합니다.
#[derive(Clone)]
pub struct ProtoRegistry {
    pool: Arc<RwLock<DescriptorPool>>,
    /// 등록된 .proto 파일 경로 목록
    file_paths: Arc<RwLock<Vec<String>>>,
}

impl ProtoRegistry {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(RwLock::new(DescriptorPool::new())),
            file_paths: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// .proto 파일들을 로드하여 레지스트리에 추가합니다.
    /// protox를 사용하여 protoc 없이 순수 Rust로 파싱합니다.
    pub async fn load_proto_files(&self, paths: &[String]) -> Result<usize, String> {
        let mut compiler = protox::Compiler::new(
            paths
                .iter()
                .filter_map(|p| Path::new(p).parent())
                .collect::<Vec<_>>(),
        )
        .map_err(|e| format!("컴파일러 초기화 실패: {}", e))?;

        for path in paths {
            let file_name = Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            compiler
                .open_file(file_name)
                .map_err(|e| format!("{} 파싱 실패: {}", path, e))?;
        }

        let file_descriptor_set = compiler.file_descriptor_set().to_owned();

        let pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set)
            .map_err(|e| format!("DescriptorPool 생성 실패: {}", e))?;

        let service_count = pool.services().count();

        *self.pool.write().await = pool;
        let mut file_paths = self.file_paths.write().await;
        for path in paths {
            if !file_paths.contains(path) {
                file_paths.push(path.clone());
            }
        }

        Ok(service_count)
    }

    /// 등록된 .proto 파일 경로 목록을 반환합니다.
    pub async fn list_files(&self) -> Vec<String> {
        self.file_paths.read().await.clone()
    }

    /// 특정 .proto 파일을 레지스트리에서 제거합니다.
    pub async fn remove_file(&self, path: &str) {
        let mut file_paths = self.file_paths.write().await;
        file_paths.retain(|p| p != path);
        // 전체 다시 로드 (간단한 구현)
        let remaining = file_paths.clone();
        drop(file_paths);
        if remaining.is_empty() {
            *self.pool.write().await = DescriptorPool::new();
        } else {
            let _ = self.load_proto_files(&remaining).await;
        }
    }

    /// gRPC 서비스/메서드 이름으로 메시지 디스크립터를 찾습니다.
    /// `service`: "package.ServiceName" 형식
    /// `method`: "MethodName" 형식
    /// `is_request`: true면 input type, false면 output type
    pub async fn find_message_descriptor(
        &self,
        service: &str,
        method: &str,
        is_request: bool,
    ) -> Option<MessageDescriptor> {
        let pool = self.pool.read().await;
        let svc = pool.services().find(|s| {
            let full_name = s.full_name();
            full_name == service || full_name.ends_with(service)
        })?;

        let m = svc.methods().find(|m| m.name() == method)?;

        Some(if is_request { m.input() } else { m.output() })
    }

    /// gRPC 바이너리 데이터를 JSON으로 디코딩합니다.
    /// gRPC 프레이밍 헤더(5바이트)를 제거한 후 protobuf 메시지를 파싱합니다.
    pub async fn decode_to_json(
        &self,
        service: &str,
        method: &str,
        data: &[u8],
        is_request: bool,
    ) -> Option<serde_json::Value> {
        let desc = self
            .find_message_descriptor(service, method, is_request)
            .await?;

        // gRPC 프레이밍: 1바이트 compressed flag + 4바이트 BE length + message
        // compressed flag가 1이면 메시지가 (보통 gzip으로) 압축돼 있으므로 해제 후 디코딩한다.
        const MAX_GRPC_DECOMPRESSED: u64 = 64 * 1024 * 1024;
        let payload: std::borrow::Cow<[u8]> = if data.len() >= 5 {
            let compressed = data[0];
            let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
            let complete = data.len() >= 5 + len;
            let frame: &[u8] = if complete { &data[5..5 + len] } else { data };
            if compressed == 1 && complete {
                use flate2::read::GzDecoder;
                use std::io::Read;
                let mut out = Vec::new();
                match GzDecoder::new(frame)
                    .take(MAX_GRPC_DECOMPRESSED)
                    .read_to_end(&mut out)
                {
                    Ok(_) => std::borrow::Cow::Owned(out),
                    Err(_) => std::borrow::Cow::Borrowed(frame),
                }
            } else {
                std::borrow::Cow::Borrowed(frame)
            }
        } else {
            std::borrow::Cow::Borrowed(data)
        };

        let msg = DynamicMessage::decode(desc, payload.as_ref()).ok()?;

        let serializer = serde_json::Serializer::new(Vec::new());
        let mut serializer = serializer;
        let opts = prost_reflect::SerializeOptions::new()
            .stringify_64_bit_integers(false)
            .use_proto_field_name(true);
        msg.serialize_with_options(&mut serializer, &opts).ok()?;

        let json_bytes = serializer.into_inner();
        serde_json::from_slice(&json_bytes).ok()
    }
}

impl Default for ProtoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_registry() {
        let registry = ProtoRegistry::new();
        assert!(registry.pool.try_read().is_ok());
    }

    #[tokio::test]
    async fn test_empty_list() {
        let registry = ProtoRegistry::new();
        let files = registry.list_files().await;
        assert!(files.is_empty());
    }
}
