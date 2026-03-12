//! Gzip 압축/해제 관련 유틸리티

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use std::path::Path;

use super::SessionError;

/// 데이터가 gzip 형식인지 매직 바이트로 확인
pub(crate) fn is_gzip(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
}

/// JSON 문자열을 gzip 압축하여 파일에 저장
pub(crate) fn save_to_file(path: &Path, json: &str) -> Result<(), SessionError> {
    let file = std::fs::File::create(path).map_err(|e| SessionError::Io(e.to_string()))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder
        .write_all(json.as_bytes())
        .map_err(|e| SessionError::Io(e.to_string()))?;
    encoder
        .finish()
        .map_err(|e| SessionError::Io(e.to_string()))?;
    Ok(())
}

/// gzip 압축된 바이트 데이터를 JSON 문자열로 해제
pub(crate) fn load_from_file(raw: &[u8]) -> Result<String, SessionError> {
    let mut decoder = GzDecoder::new(raw);
    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .map_err(|e| SessionError::Decompress(e.to_string()))?;
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_gzip_valid() {
        assert!(is_gzip(&[0x1f, 0x8b, 0x08]));
        assert!(is_gzip(&[0x1f, 0x8b]));
    }

    #[test]
    fn test_is_gzip_invalid() {
        // JSON은 '{'로 시작
        assert!(!is_gzip(&[0x7b, 0x22]));
        // 너무 짧음
        assert!(!is_gzip(&[0x1f]));
        // 빈 데이터
        assert!(!is_gzip(&[]));
        // 첫 바이트만 일치
        assert!(!is_gzip(&[0x1f, 0x00]));
    }

    #[test]
    fn test_gzip_roundtrip() {
        // 압축 후 해제하면 원본과 동일해야 함
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.gz");
        let original = r#"{"hello": "world", "number": 42}"#;

        save_to_file(&path, original).unwrap();

        let raw = std::fs::read(&path).unwrap();
        assert!(is_gzip(&raw), "저장된 파일은 gzip 형식이어야 함");

        let decompressed = load_from_file(&raw).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_gzip_roundtrip_large_data() {
        // 큰 데이터에서도 정상 동작 확인
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("large.gz");
        let original: String = (0..10000).map(|i| format!("item_{} ", i)).collect();

        save_to_file(&path, &original).unwrap();
        let raw = std::fs::read(&path).unwrap();
        let decompressed = load_from_file(&raw).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_gzip_compressed_smaller_than_original() {
        // 반복 데이터는 압축 시 더 작아야 함
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("compress.gz");
        let original = "abcdefghij".repeat(1000);

        save_to_file(&path, &original).unwrap();
        let compressed_size = std::fs::metadata(&path).unwrap().len();
        let original_size = original.len() as u64;

        assert!(
            compressed_size < original_size,
            "압축 크기({compressed_size})가 원본 크기({original_size})보다 작아야 함"
        );
    }

    #[test]
    fn test_load_from_file_invalid_gzip() {
        // 잘못된 gzip 데이터 해제 시 에러 반환
        let invalid_data = [0x1f, 0x8b, 0x00, 0x00, 0xff, 0xff];
        let result = load_from_file(&invalid_data);
        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::Decompress(_) => {} // 예상대로
            other => panic!("Decompress 에러를 기대했지만 {:?}를 받음", other),
        }
    }

    #[test]
    fn test_gzip_roundtrip_empty_data() {
        // 빈 문자열도 정상적으로 압축/해제 가능해야 함
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.gz");

        save_to_file(&path, "").unwrap();
        let raw = std::fs::read(&path).unwrap();
        assert!(is_gzip(&raw), "빈 데이터도 gzip 형식이어야 함");

        let decompressed = load_from_file(&raw).unwrap();
        assert_eq!(decompressed, "");
    }

    #[test]
    fn test_save_to_file_invalid_path() {
        let result = save_to_file(std::path::Path::new("/nonexistent/dir/file.gz"), "data");
        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::Io(_) => {}
            other => panic!("Io 에러를 기대했지만 {:?}를 받음", other),
        }
    }
}
