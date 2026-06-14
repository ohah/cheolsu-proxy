use std::fmt;
use std::net::SocketAddr;

// SOCKS5 프로토콜 상수 (RFC 1928)

/// SOCKS 프로토콜 버전
pub const SOCKS_VERSION: u8 = 0x05;

// --- 인증 방법 ---
/// 인증 없음
pub const AUTH_NO_AUTH: u8 = 0x00;
/// 사용자명/비밀번호 (RFC 1929)
pub const AUTH_USERNAME_PASSWORD: u8 = 0x02;
/// 수용 가능한 방법 없음
pub const AUTH_NO_ACCEPTABLE: u8 = 0xFF;

// --- 명령 ---
/// TCP CONNECT
pub const CMD_CONNECT: u8 = 0x01;
/// TCP BIND (미지원)
pub const CMD_BIND: u8 = 0x02;
/// UDP ASSOCIATE (미지원)
pub const CMD_UDP_ASSOCIATE: u8 = 0x03;

// --- 주소 타입 ---
/// IPv4
pub const ATYP_IPV4: u8 = 0x01;
/// 도메인 이름
pub const ATYP_DOMAIN: u8 = 0x03;
/// IPv6
pub const ATYP_IPV6: u8 = 0x04;

// --- 응답 코드 ---
/// 성공
pub const REPLY_SUCCEEDED: u8 = 0x00;
/// 일반 SOCKS 서버 실패
pub const REPLY_GENERAL_FAILURE: u8 = 0x01;
/// 규칙에 의해 허용되지 않음
pub const REPLY_NOT_ALLOWED: u8 = 0x02;
/// 네트워크 도달 불가
pub const REPLY_NETWORK_UNREACHABLE: u8 = 0x03;
/// 호스트 도달 불가
pub const REPLY_HOST_UNREACHABLE: u8 = 0x04;
/// 연결 거부
pub const REPLY_CONNECTION_REFUSED: u8 = 0x05;
/// TTL 만료
pub const REPLY_TTL_EXPIRED: u8 = 0x06;
/// 지원하지 않는 명령
pub const REPLY_COMMAND_NOT_SUPPORTED: u8 = 0x07;
/// 지원하지 않는 주소 타입
pub const REPLY_ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;

/// 클라이언트 인사 메시지
#[derive(Debug)]
pub struct Socks5Greeting {
    pub methods: Vec<u8>,
}

/// SOCKS5 대상 주소
#[derive(Debug, Clone, PartialEq)]
pub enum TargetAddr {
    /// IPv4 주소와 포트
    Ipv4([u8; 4], u16),
    /// 도메인 이름과 포트
    Domain(String, u16),
    /// IPv6 주소와 포트
    Ipv6([u8; 16], u16),
}

impl TargetAddr {
    /// "host:port" 형식의 문자열로 변환합니다.
    pub fn to_address_string(&self) -> String {
        match self {
            TargetAddr::Ipv4(addr, port) => {
                format!("{}.{}.{}.{}:{}", addr[0], addr[1], addr[2], addr[3], port)
            }
            TargetAddr::Domain(domain, port) => {
                format!("{}:{}", domain, port)
            }
            TargetAddr::Ipv6(addr, port) => {
                let segments: Vec<u16> = addr
                    .chunks(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();

                // 간략화된 IPv6 표현
                let ipv6 = std::net::Ipv6Addr::new(
                    segments[0],
                    segments[1],
                    segments[2],
                    segments[3],
                    segments[4],
                    segments[5],
                    segments[6],
                    segments[7],
                );
                // IPv6 리터럴은 RFC 3986/7230 authority-form에서 대괄호로 감싸야 한다
                // (예: [::1]:443). 비대괄호 형태는 host:port 파싱과 CONNECT 요청을 깨뜨린다.
                format!("[{}]:{}", ipv6, port)
            }
        }
    }

    /// SocketAddr에서 TargetAddr를 생성합니다.
    pub fn from_socket_addr(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(v4) => TargetAddr::Ipv4(v4.ip().octets(), v4.port()),
            SocketAddr::V6(v6) => TargetAddr::Ipv6(v6.ip().octets(), v6.port()),
        }
    }
}

impl fmt::Display for TargetAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_address_string())
    }
}

/// SOCKS5 요청
#[derive(Debug)]
pub struct Socks5Request {
    pub command: u8,
    pub target: TargetAddr,
}

/// SOCKS5 에러 타입
#[derive(Debug, thiserror::Error)]
pub enum Socks5Error {
    #[error("IO 에러: {0}")]
    Io(std::io::Error),

    #[error("잘못된 SOCKS 버전: {0}")]
    InvalidVersion(u8),

    #[error("인증 방법이 제공되지 않음")]
    NoMethodsProvided,

    #[error("수용 가능한 인증 방법 없음")]
    NoAcceptableAuthMethod,

    #[error("잘못된 서브네고시에이션 버전: {0}")]
    InvalidSubnegotiationVersion(u8),

    #[error("인증 실패")]
    AuthenticationFailed,

    #[error("지원하지 않는 명령: 0x{0:02x}")]
    UnsupportedCommand(u8),

    #[error("잘못된 주소 타입: 0x{0:02x}")]
    InvalidAddressType(u8),

    #[error("잘못된 도메인 이름")]
    InvalidDomainName,

    #[error("Upstream 연결 실패: {0}")]
    UpstreamConnect(#[from] crate::upstream_proxy::UpstreamProxyError),
}
