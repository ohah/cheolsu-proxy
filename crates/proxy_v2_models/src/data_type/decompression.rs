use brotli::Decompressor;
use flate2::read::GzDecoder;
use std::io::Read;

/// 압축 해제 결과의 최대 허용 크기(압축 폭탄 방지). 100 MiB.
pub const MAX_DECOMPRESSED_SIZE: u64 = 100 * 1024 * 1024;

/// 상한을 적용해 reader를 모두 읽는다. 상한 초과 시 에러를 반환한다.
fn read_to_end_capped<R: Read>(reader: R) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut decompressed = Vec::new();
    // MAX+1 바이트까지 읽어, MAX를 초과하면(=MAX+1 도달) 폭탄으로 간주한다.
    reader
        .take(MAX_DECOMPRESSED_SIZE + 1)
        .read_to_end(&mut decompressed)?;
    if decompressed.len() as u64 > MAX_DECOMPRESSED_SIZE {
        return Err(format!(
            "압축 해제 크기가 한도({} bytes)를 초과했습니다(압축 폭탄 의심)",
            MAX_DECOMPRESSED_SIZE
        )
        .into());
    }
    Ok(decompressed)
}

/// GZIP 압축 해제 함수
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    read_to_end_capped(GzDecoder::new(data))
}

/// Brotli 압축 해제 함수
pub fn decompress_brotli(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    read_to_end_capped(Decompressor::new(data, 4096))
}
