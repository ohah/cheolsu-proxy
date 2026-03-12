use super::super::DataType;

/// 이미지 파일 시그니처 감지 (PNG, JPEG, GIF, WebP, BMP, ICO, TIFF)
pub(crate) fn detect_image_signature(body: &[u8]) -> Option<DataType> {
    if body.len() < 2 {
        return None;
    }

    // PNG 시그니처 (매우 명확)
    if body.len() >= 8 && &body[0..8] == b"\x89PNG\r\n\x1a\n" {
        return Some(DataType::Image);
    }

    // JPEG 시그니처 (매우 명확)
    if &body[0..2] == b"\xff\xd8" {
        return Some(DataType::Image);
    }

    // GIF 시그니처 (매우 명확)
    if body.len() >= 6 && (&body[0..6] == b"GIF87a" || &body[0..6] == b"GIF89a") {
        return Some(DataType::Image);
    }

    // WebP 시그니처 (RIFF 컨테이너 확인)
    if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WEBP" {
        return Some(DataType::Image);
    }

    // BMP 시그니처
    if body.len() >= 2 && &body[0..2] == b"BM" {
        return Some(DataType::Image);
    }

    // ICO 시그니처
    if body.len() >= 4 && &body[0..4] == b"\x00\x00\x01\x00" {
        return Some(DataType::Image);
    }

    // TIFF 시그니처 (Little Endian)
    if body.len() >= 4 && &body[0..4] == b"II*\x00" {
        return Some(DataType::Image);
    }

    // TIFF 시그니처 (Big Endian)
    if body.len() >= 4 && &body[0..4] == b"MM\x00*" {
        return Some(DataType::Image);
    }

    None
}

/// ftyp 기반 컨테이너 감지 (이미지/오디오/비디오 통합)
pub(crate) fn detect_ftyp_signature(body: &[u8]) -> Option<DataType> {
    if body.len() < 12 || &body[4..8] != b"ftyp" {
        return None;
    }

    let brand = &body[8..12];
    // 이미지: AVIF/HEIF/HEIC
    if brand == b"avif"
        || brand == b"avis"
        || brand == b"heic"
        || brand == b"heix"
        || brand == b"mif1"
    {
        return Some(DataType::Image);
    }
    // 오디오: M4A/AAC
    if brand == b"M4A " || brand == b"M4B " || brand == b"mp4a" {
        return Some(DataType::Audio);
    }
    // 비디오: MP4/M4V/3GP 등
    if brand == b"mp41"
        || brand == b"mp42"
        || brand == b"isom"
        || brand == b"avc1"
        || brand == b"iso2"
        || brand == b"iso3"
        || brand == b"iso4"
        || brand == b"iso5"
        || brand == b"iso6"
        || brand == b"M4V "
        || brand == b"M4VP"
        || brand == b"3gp4"
        || brand == b"3gp5"
        || brand == b"3gp6"
        || brand == b"3g2a"
        || brand == b"dash"
        || brand == b"mmp4"
    {
        return Some(DataType::Video);
    }
    // ftyp가 있지만 알 수 없는 브랜드 → 비디오로 추정 (안전한 폴백)
    Some(DataType::Video)
}

/// 비디오 파일 시그니처 감지 (ftyp 이외: MOV, WebM, AVI, FLV, MPEG-TS)
pub(crate) fn detect_video_signature(body: &[u8]) -> Option<DataType> {
    if body.len() >= 8 {
        // MOV 파일 시그니처 (QuickTime)
        if &body[4..8] == b"moov"
            || &body[4..8] == b"mdat"
            || &body[4..8] == b"free"
            || &body[4..8] == b"wide"
        {
            return Some(DataType::Video);
        }
    }

    if body.len() >= 4 {
        // WebM/MKV 시그니처 (EBML 매직 넘버)
        if &body[0..4] == b"\x1a\x45\xdf\xa3" {
            return Some(DataType::Video);
        }
        // AVI 시그니처 (RIFF 컨테이너)
        if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"AVI " {
            return Some(DataType::Video);
        }
        // FLV 시그니처
        if &body[0..3] == b"FLV" && body[3] == 0x01 {
            return Some(DataType::Video);
        }
        // MPEG-TS 시그니처 (동기화 바이트 0x47, 188바이트 패킷)
        if body[0] == 0x47 && body.len() >= 188 + 4 && body[188] == 0x47 {
            return Some(DataType::Video);
        }
    }

    None
}

/// 오디오 파일 시그니처 감지 (ftyp 이외: MP3, WAV, OGG, FLAC, AIFF)
pub(crate) fn detect_audio_signature(body: &[u8]) -> Option<DataType> {
    if body.len() < 2 {
        return None;
    }

    // MP3 시그니처 (ID3 태그)
    if body.len() >= 3 && &body[0..3] == b"ID3" {
        return Some(DataType::Audio);
    }
    // MP3 프레임 동기 (0xFF + 상위 3비트 = 111)
    if body[0] == 0xFF && (body[1] & 0xE0) == 0xE0 {
        return Some(DataType::Audio);
    }
    // WAV 시그니처 (RIFF 컨테이너)
    if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WAVE" {
        return Some(DataType::Audio);
    }
    // OGG 시그니처 (Vorbis, Opus 등)
    if body.len() >= 4 && &body[0..4] == b"OggS" {
        return Some(DataType::Audio);
    }
    // FLAC 시그니처
    if body.len() >= 4 && &body[0..4] == b"fLaC" {
        return Some(DataType::Audio);
    }
    // AIFF 시그니처
    if body.len() >= 12 && &body[0..4] == b"FORM" && &body[8..12] == b"AIFF" {
        return Some(DataType::Audio);
    }

    None
}

/// 문서/아카이브 파일 시그니처 감지 (PDF, ZIP)
pub(crate) fn detect_document_archive_signature(body: &[u8]) -> Option<DataType> {
    if body.len() < 4 {
        return None;
    }

    // 문서 파일 감지
    if &body[0..4] == b"%PDF" {
        return Some(DataType::Document);
    }

    // ZIP 아카이브 감지 (GZIP이 아닌 경우)
    if &body[0..4] == b"PK\x03\x04" {
        return Some(DataType::Archive);
    }

    None
}
