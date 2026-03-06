/// mitmproxy 스타일 Flow Filter 표현식 파서 및 평가기
///
/// 지원하는 필터:
///   ~u regex     URL 매칭
///   ~m regex     HTTP 메서드 매칭
///   ~d regex     도메인 매칭
///   ~bq regex    요청 본문(body) 매칭
///   ~bs regex    응답 본문 매칭
///   ~b regex     요청+응답 본문 매칭
///   ~hq regex    요청 헤더 매칭 ("Name: Value" 형식)
///   ~hs regex    응답 헤더 매칭
///   ~h regex     요청+응답 헤더 매칭
///   ~tq regex    요청 Content-Type 매칭
///   ~ts regex    응답 Content-Type 매칭
///   ~t regex     요청+응답 Content-Type 매칭
///   ~c code      응답 상태코드 매칭 (정확히 일치)
///   ~q           요청만 (응답 없는 상태)
///   ~s           응답 있는 상태
///   ~all         모든 플로우
///
/// 연산자:
///   !expr        NOT
///   expr & expr  AND
///   expr | expr  OR
///   (...)        그룹핑
///
/// 예시:
///   ~u "api/login" & ~m POST & ~bq "username=admin"
///   ~d example.com & ~tq json
///   !~u "healthcheck"
use proxyapi_v2::hyper::http::HeaderMap;
use regex::Regex;

/// 필터 평가 시 사용하는 플로우 컨텍스트
pub struct FlowContext<'a> {
    pub url: &'a str,
    pub method: &'a str,
    pub request_headers: &'a HeaderMap,
    pub request_body: &'a [u8],
    pub response_status: Option<u16>,
    pub response_headers: Option<&'a HeaderMap>,
    pub response_body: Option<&'a [u8]>,
}

/// 컴파일된 필터 표현식
pub struct FlowFilter {
    expr: FilterExpr,
}

impl FlowFilter {
    /// 필터 표현식 문자열을 파싱하여 컴파일
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        if input.is_empty() || input == "~all" {
            return Ok(FlowFilter {
                expr: FilterExpr::All,
            });
        }

        let tokens = tokenize(input)?;
        if tokens.is_empty() {
            return Ok(FlowFilter {
                expr: FilterExpr::All,
            });
        }

        let mut pos = 0;
        let expr = parse_or(&tokens, &mut pos)?;
        if pos < tokens.len() {
            return Err(format!(
                "예상치 못한 토큰: {:?} (위치 {})",
                tokens[pos], pos
            ));
        }
        Ok(FlowFilter { expr })
    }

    /// 필터를 FlowContext에 대해 평가
    pub fn matches(&self, ctx: &FlowContext) -> bool {
        self.expr.matches(ctx)
    }
}

// --- AST ---

enum FilterExpr {
    All,
    IsRequest,
    IsResponse,
    Url(Regex),
    Method(Regex),
    Domain(Regex),
    Body(Regex),
    BodyRequest(Regex),
    BodyResponse(Regex),
    Header(Regex),
    HeaderRequest(Regex),
    HeaderResponse(Regex),
    ContentType(Regex),
    ContentTypeRequest(Regex),
    ContentTypeResponse(Regex),
    StatusCode(u16),
    Not(Box<FilterExpr>),
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
}

impl FilterExpr {
    fn matches(&self, ctx: &FlowContext) -> bool {
        match self {
            FilterExpr::All => true,
            FilterExpr::IsRequest => ctx.response_status.is_none(),
            FilterExpr::IsResponse => ctx.response_status.is_some(),

            FilterExpr::Url(re) => re.is_match(ctx.url),
            FilterExpr::Method(re) => re.is_match(ctx.method),
            FilterExpr::Domain(re) => {
                let domain = extract_domain(ctx.url);
                re.is_match(&domain)
            }

            FilterExpr::Body(re) => {
                match_body(re, ctx.request_body)
                    || ctx.response_body.map_or(false, |b| match_body(re, b))
            }
            FilterExpr::BodyRequest(re) => match_body(re, ctx.request_body),
            FilterExpr::BodyResponse(re) => ctx.response_body.map_or(false, |b| match_body(re, b)),

            FilterExpr::Header(re) => {
                match_headers(re, ctx.request_headers)
                    || ctx.response_headers.map_or(false, |h| match_headers(re, h))
            }
            FilterExpr::HeaderRequest(re) => match_headers(re, ctx.request_headers),
            FilterExpr::HeaderResponse(re) => {
                ctx.response_headers.map_or(false, |h| match_headers(re, h))
            }

            FilterExpr::ContentType(re) => {
                match_content_type(re, ctx.request_headers)
                    || ctx
                        .response_headers
                        .map_or(false, |h| match_content_type(re, h))
            }
            FilterExpr::ContentTypeRequest(re) => match_content_type(re, ctx.request_headers),
            FilterExpr::ContentTypeResponse(re) => ctx
                .response_headers
                .map_or(false, |h| match_content_type(re, h)),

            FilterExpr::StatusCode(code) => ctx.response_status == Some(*code),

            FilterExpr::Not(inner) => !inner.matches(ctx),
            FilterExpr::And(left, right) => left.matches(ctx) && right.matches(ctx),
            FilterExpr::Or(left, right) => left.matches(ctx) || right.matches(ctx),
        }
    }
}

// --- 헬퍼 함수 ---

fn extract_domain(url: &str) -> String {
    // "https://api.example.com:443/path" -> "api.example.com"
    // "api.example.com:443" -> "api.example.com"
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host_part = without_scheme.split('/').next().unwrap_or(without_scheme);
    host_part.split(':').next().unwrap_or(host_part).to_string()
}

fn match_body(re: &Regex, body: &[u8]) -> bool {
    if body.is_empty() {
        return false;
    }
    // UTF-8로 시도, 실패하면 lossy 변환
    match std::str::from_utf8(body) {
        Ok(s) => re.is_match(s),
        Err(_) => re.is_match(&String::from_utf8_lossy(body)),
    }
}

fn match_headers(re: &Regex, headers: &HeaderMap) -> bool {
    for (name, value) in headers.iter() {
        let header_line = format!("{}: {}", name.as_str(), value.to_str().unwrap_or(""));
        if re.is_match(&header_line) {
            return true;
        }
    }
    false
}

fn match_content_type(re: &Regex, headers: &HeaderMap) -> bool {
    headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map_or(false, |ct| re.is_match(ct))
}

// --- 토크나이저 ---

#[derive(Debug, Clone)]
enum Token {
    FilterKeyword(FilterType),
    Pattern(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
}

#[derive(Debug, Clone)]
enum FilterType {
    Url,
    Method,
    Domain,
    Body,
    BodyRequest,
    BodyResponse,
    Header,
    HeaderRequest,
    HeaderResponse,
    ContentType,
    ContentTypeRequest,
    ContentTypeResponse,
    StatusCode,
    IsRequest,
    IsResponse,
    All,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // 공백 스킵
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }

        match chars[i] {
            '&' => {
                tokens.push(Token::And);
                i += 1;
            }
            '|' => {
                tokens.push(Token::Or);
                i += 1;
            }
            '!' => {
                tokens.push(Token::Not);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '~' => {
                // 필터 키워드 파싱
                let (keyword, new_i) = parse_filter_keyword(&chars, i)?;
                tokens.push(Token::FilterKeyword(keyword));
                i = new_i;
            }
            '"' => {
                // 따옴표 문자열
                let (s, new_i) = parse_quoted_string(&chars, i)?;
                tokens.push(Token::Pattern(s));
                i = new_i;
            }
            _ => {
                // 일반 단어 (공백, 연산자, 괄호까지)
                let (word, new_i) = parse_word(&chars, i);
                tokens.push(Token::Pattern(word));
                i = new_i;
            }
        }
    }

    Ok(tokens)
}

fn parse_filter_keyword(chars: &[char], start: usize) -> Result<(FilterType, usize), String> {
    let remaining: String = chars[start..].iter().collect();

    // 긴 키워드부터 매칭 (bq before b, hq before h, etc.)
    let keywords: &[(&str, FilterType)] = &[
        ("~bq", FilterType::BodyRequest),
        ("~bs", FilterType::BodyResponse),
        ("~hq", FilterType::HeaderRequest),
        ("~hs", FilterType::HeaderResponse),
        ("~tq", FilterType::ContentTypeRequest),
        ("~ts", FilterType::ContentTypeResponse),
        ("~all", FilterType::All),
        ("~u", FilterType::Url),
        ("~m", FilterType::Method),
        ("~d", FilterType::Domain),
        ("~b", FilterType::Body),
        ("~h", FilterType::Header),
        ("~t", FilterType::ContentType),
        ("~c", FilterType::StatusCode),
        ("~q", FilterType::IsRequest),
        ("~s", FilterType::IsResponse),
    ];

    for (prefix, filter_type) in keywords {
        if remaining.starts_with(prefix) {
            let end = start + prefix.len();
            // 키워드 뒤에 단어 문자가 이어지면 안 됨 (예: ~url은 ~u + "rl"이 아님)
            if end < chars.len() && chars[end].is_alphanumeric() && prefix.len() <= 2 {
                continue;
            }
            return Ok((filter_type.clone(), end));
        }
    }

    Err(format!(
        "알 수 없는 필터 키워드: {}",
        &remaining[..remaining.len().min(10)]
    ))
}

fn parse_quoted_string(chars: &[char], start: usize) -> Result<(String, usize), String> {
    let mut i = start + 1; // 여는 따옴표 스킵
    let mut result = String::new();

    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            result.push(chars[i + 1]);
            i += 2;
        } else if chars[i] == '"' {
            return Ok((result, i + 1));
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    Err("닫히지 않은 따옴표".to_string())
}

fn parse_word(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    let mut word = String::new();

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '&' | '|' | '!' | '(' | ')' => break,
            c => {
                word.push(c);
                i += 1;
            }
        }
    }

    (word, i)
}

// --- 파서 (재귀 하향) ---

fn build_regex(pattern: &str) -> Result<Regex, String> {
    // 대소문자 무시 플래그 추가
    let pattern = format!("(?i){}", pattern);
    Regex::new(&pattern).map_err(|e| format!("정규식 오류 '{}': {}", pattern, e))
}

fn expect_pattern(tokens: &[Token], pos: &mut usize) -> Result<String, String> {
    if *pos >= tokens.len() {
        return Err("패턴이 필요하지만 입력이 끝남".to_string());
    }
    match &tokens[*pos] {
        Token::Pattern(s) => {
            *pos += 1;
            Ok(s.clone())
        }
        other => Err(format!("패턴이 필요하지만 {:?}을(를) 발견", other)),
    }
}

fn parse_or(tokens: &[Token], pos: &mut usize) -> Result<FilterExpr, String> {
    let mut left = parse_and(tokens, pos)?;

    while *pos < tokens.len() {
        if matches!(tokens[*pos], Token::Or) {
            *pos += 1;
            let right = parse_and(tokens, pos)?;
            left = FilterExpr::Or(Box::new(left), Box::new(right));
        } else {
            break;
        }
    }

    Ok(left)
}

fn parse_and(tokens: &[Token], pos: &mut usize) -> Result<FilterExpr, String> {
    let mut left = parse_not(tokens, pos)?;

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::And => {
                *pos += 1;
                let right = parse_not(tokens, pos)?;
                left = FilterExpr::And(Box::new(left), Box::new(right));
            }
            // 묵시적 AND: 두 atom이 연속되면 AND로 처리
            Token::FilterKeyword(_) | Token::Pattern(_) | Token::Not | Token::LParen => {
                let right = parse_not(tokens, pos)?;
                left = FilterExpr::And(Box::new(left), Box::new(right));
            }
            _ => break,
        }
    }

    Ok(left)
}

fn parse_not(tokens: &[Token], pos: &mut usize) -> Result<FilterExpr, String> {
    if *pos < tokens.len() && matches!(tokens[*pos], Token::Not) {
        *pos += 1;
        let inner = parse_not(tokens, pos)?;
        return Ok(FilterExpr::Not(Box::new(inner)));
    }
    parse_atom(tokens, pos)
}

fn parse_atom(tokens: &[Token], pos: &mut usize) -> Result<FilterExpr, String> {
    if *pos >= tokens.len() {
        return Err("표현식이 필요하지만 입력이 끝남".to_string());
    }

    match &tokens[*pos] {
        Token::LParen => {
            *pos += 1;
            let expr = parse_or(tokens, pos)?;
            if *pos >= tokens.len() || !matches!(tokens[*pos], Token::RParen) {
                return Err("닫는 괄호 ')' 필요".to_string());
            }
            *pos += 1;
            Ok(expr)
        }
        Token::FilterKeyword(ft) => {
            let ft = ft.clone();
            *pos += 1;
            match ft {
                FilterType::All => Ok(FilterExpr::All),
                FilterType::IsRequest => Ok(FilterExpr::IsRequest),
                FilterType::IsResponse => Ok(FilterExpr::IsResponse),

                FilterType::Url => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::Url(build_regex(&pattern)?))
                }
                FilterType::Method => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::Method(build_regex(&pattern)?))
                }
                FilterType::Domain => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::Domain(build_regex(&pattern)?))
                }
                FilterType::Body => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::Body(build_regex(&pattern)?))
                }
                FilterType::BodyRequest => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::BodyRequest(build_regex(&pattern)?))
                }
                FilterType::BodyResponse => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::BodyResponse(build_regex(&pattern)?))
                }
                FilterType::Header => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::Header(build_regex(&pattern)?))
                }
                FilterType::HeaderRequest => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::HeaderRequest(build_regex(&pattern)?))
                }
                FilterType::HeaderResponse => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::HeaderResponse(build_regex(&pattern)?))
                }
                FilterType::ContentType => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::ContentType(build_regex(&pattern)?))
                }
                FilterType::ContentTypeRequest => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::ContentTypeRequest(build_regex(&pattern)?))
                }
                FilterType::ContentTypeResponse => {
                    let pattern = expect_pattern(tokens, pos)?;
                    Ok(FilterExpr::ContentTypeResponse(build_regex(&pattern)?))
                }
                FilterType::StatusCode => {
                    let pattern = expect_pattern(tokens, pos)?;
                    let code: u16 = pattern
                        .parse()
                        .map_err(|_| format!("유효하지 않은 상태 코드: {}", pattern))?;
                    Ok(FilterExpr::StatusCode(code))
                }
            }
        }
        Token::Pattern(p) => {
            // bare 패턴 → URL 매칭 (mitmproxy 동작과 동일)
            let pattern = p.clone();
            *pos += 1;
            Ok(FilterExpr::Url(build_regex(&pattern)?))
        }
        other => Err(format!("예상치 못한 토큰: {:?}", other)),
    }
}

// --- 테스트 ---

#[cfg(test)]
mod tests {
    use super::*;
    use proxyapi_v2::hyper::http::HeaderMap;

    fn make_ctx<'a>(
        url: &'a str,
        method: &'a str,
        req_headers: &'a HeaderMap,
        req_body: &'a [u8],
        res_status: Option<u16>,
        res_headers: Option<&'a HeaderMap>,
        res_body: Option<&'a [u8]>,
    ) -> FlowContext<'a> {
        FlowContext {
            url,
            method,
            request_headers: req_headers,
            request_body: req_body,
            response_status: res_status,
            response_headers: res_headers,
            response_body: res_body,
        }
    }

    fn empty_headers() -> HeaderMap {
        HeaderMap::new()
    }

    #[test]
    fn test_parse_all() {
        let f = FlowFilter::parse("~all").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://example.com", "GET", &h, b"", None, None, None);
        assert!(f.matches(&ctx));
    }

    #[test]
    fn test_parse_empty() {
        let f = FlowFilter::parse("").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://example.com", "GET", &h, b"", None, None, None);
        assert!(f.matches(&ctx));
    }

    #[test]
    fn test_url_match() {
        let f = FlowFilter::parse("~u example.com").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://example.com/api", "GET", &h, b"", None, None, None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx("https://other.com/api", "GET", &h, b"", None, None, None);
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_bare_pattern_is_url() {
        let f = FlowFilter::parse("example.com").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://example.com/api", "GET", &h, b"", None, None, None);
        assert!(f.matches(&ctx));
    }

    #[test]
    fn test_method_match() {
        let f = FlowFilter::parse("~m POST").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://example.com", "POST", &h, b"", None, None, None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx("https://example.com", "GET", &h, b"", None, None, None);
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_domain_match() {
        let f = FlowFilter::parse("~d api\\.example\\.com").unwrap();
        let h = empty_headers();
        let ctx = make_ctx(
            "https://api.example.com/v1/users",
            "GET",
            &h,
            b"",
            None,
            None,
            None,
        );
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx(
            "https://other.com/api.example.com",
            "GET",
            &h,
            b"",
            None,
            None,
            None,
        );
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_body_request_match() {
        let f = FlowFilter::parse(r#"~bq "action.*delete""#).unwrap();
        let h = empty_headers();
        let body = br#"{"action":"delete","id":123}"#;
        let ctx = make_ctx("https://api.com", "POST", &h, body, None, None, None);
        assert!(f.matches(&ctx));

        let body2 = br#"{"action":"list"}"#;
        let ctx2 = make_ctx("https://api.com", "POST", &h, body2, None, None, None);
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_body_response_match() {
        let f = FlowFilter::parse(r#"~bs "success.*true""#).unwrap();
        let h = empty_headers();
        let res_body = br#"{"success": true}"#;
        let ctx = make_ctx(
            "https://api.com",
            "GET",
            &h,
            b"",
            Some(200),
            Some(&h),
            Some(res_body),
        );
        assert!(f.matches(&ctx));
    }

    #[test]
    fn test_header_request_match() {
        let f = FlowFilter::parse(r#"~hq "authorization: Bearer""#).unwrap();
        let mut h = HeaderMap::new();
        h.insert("authorization", "Bearer token123".parse().unwrap());
        let ctx = make_ctx("https://api.com", "GET", &h, b"", None, None, None);
        assert!(f.matches(&ctx));
    }

    #[test]
    fn test_content_type_match() {
        let f = FlowFilter::parse("~tq json").unwrap();
        let mut h = HeaderMap::new();
        h.insert("content-type", "application/json".parse().unwrap());
        let ctx = make_ctx("https://api.com", "POST", &h, b"{}", None, None, None);
        assert!(f.matches(&ctx));
    }

    #[test]
    fn test_status_code_match() {
        let f = FlowFilter::parse("~c 200").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://api.com", "GET", &h, b"", Some(200), Some(&h), None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx("https://api.com", "GET", &h, b"", Some(404), Some(&h), None);
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_is_request() {
        let f = FlowFilter::parse("~q").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://api.com", "GET", &h, b"", None, None, None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx("https://api.com", "GET", &h, b"", Some(200), Some(&h), None);
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_and_operator() {
        let f = FlowFilter::parse("~u api & ~m POST").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://api.com", "POST", &h, b"", None, None, None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx("https://api.com", "GET", &h, b"", None, None, None);
        assert!(!f.matches(&ctx2));

        let ctx3 = make_ctx("https://other.com", "POST", &h, b"", None, None, None);
        assert!(!f.matches(&ctx3));
    }

    #[test]
    fn test_or_operator() {
        let f = FlowFilter::parse("~m GET | ~m POST").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://api.com", "GET", &h, b"", None, None, None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx("https://api.com", "DELETE", &h, b"", None, None, None);
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_not_operator() {
        let f = FlowFilter::parse("!~u healthcheck").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://api.com/users", "GET", &h, b"", None, None, None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx(
            "https://api.com/healthcheck",
            "GET",
            &h,
            b"",
            None,
            None,
            None,
        );
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_grouping() {
        let f = FlowFilter::parse("(~d api.com | ~d staging.com) & ~m POST").unwrap();
        let h = empty_headers();

        let ctx = make_ctx("https://api.com/v1", "POST", &h, b"", None, None, None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx("https://staging.com/v1", "POST", &h, b"", None, None, None);
        assert!(f.matches(&ctx2));

        let ctx3 = make_ctx("https://api.com/v1", "GET", &h, b"", None, None, None);
        assert!(!f.matches(&ctx3));

        let ctx4 = make_ctx("https://other.com/v1", "POST", &h, b"", None, None, None);
        assert!(!f.matches(&ctx4));
    }

    #[test]
    fn test_implicit_and() {
        // mitmproxy처럼 & 없이 공백으로 연결하면 묵시적 AND
        let f = FlowFilter::parse("~u api ~m POST").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://api.com", "POST", &h, b"", None, None, None);
        assert!(f.matches(&ctx));

        let ctx2 = make_ctx("https://api.com", "GET", &h, b"", None, None, None);
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_complex_filter() {
        // 실전 예시: POST 요청이면서 body에 delete가 포함된 API 호출
        let f = FlowFilter::parse(r#"~u "/api/v1" & ~m POST & ~bq "action.*delete""#).unwrap();
        let h = empty_headers();

        let body = br#"{"action":"delete","id":42}"#;
        let ctx = make_ctx(
            "https://example.com/api/v1/items",
            "POST",
            &h,
            body,
            None,
            None,
            None,
        );
        assert!(f.matches(&ctx));

        let body2 = br#"{"action":"list"}"#;
        let ctx2 = make_ctx(
            "https://example.com/api/v1/items",
            "POST",
            &h,
            body2,
            None,
            None,
            None,
        );
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_case_insensitive() {
        let f = FlowFilter::parse("~m post").unwrap();
        let h = empty_headers();
        let ctx = make_ctx("https://api.com", "POST", &h, b"", None, None, None);
        assert!(f.matches(&ctx));
    }

    #[test]
    fn test_quoted_pattern() {
        let f = FlowFilter::parse(r#"~u "example\.com/api""#).unwrap();
        let h = empty_headers();
        let ctx = make_ctx(
            "https://example.com/api/v1",
            "GET",
            &h,
            b"",
            None,
            None,
            None,
        );
        assert!(f.matches(&ctx));
    }

    #[test]
    fn test_invalid_filter() {
        assert!(FlowFilter::parse("~z invalid").is_err());
        assert!(FlowFilter::parse("~u").is_err());
        assert!(FlowFilter::parse("~c notanumber").is_err());
        assert!(FlowFilter::parse("(~u api").is_err());
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://api.example.com/path"),
            "api.example.com"
        );
        assert_eq!(
            extract_domain("https://api.example.com:443/path"),
            "api.example.com"
        );
        assert_eq!(extract_domain("http://localhost:8080"), "localhost");
        assert_eq!(extract_domain("api.example.com:443"), "api.example.com");
    }

    #[test]
    fn test_payload_based_distinction() {
        // 핵심 시나리오: 동일 URL, 다른 payload 구분
        let f = FlowFilter::parse(r#"~u "/api" & ~bq "username=admin""#).unwrap();
        let h = empty_headers();

        // admin 로그인 → 매칭
        let ctx1 = make_ctx(
            "https://example.com/api/login",
            "POST",
            &h,
            b"username=admin&password=secret",
            None,
            None,
            None,
        );
        assert!(f.matches(&ctx1));

        // 일반 사용자 → 매칭 안 됨
        let ctx2 = make_ctx(
            "https://example.com/api/login",
            "POST",
            &h,
            b"username=john&password=1234",
            None,
            None,
            None,
        );
        assert!(!f.matches(&ctx2));
    }

    #[test]
    fn test_json_body_distinction() {
        // JSON body에서 특정 필드 구분
        let f = FlowFilter::parse(r#"~u "/api" & ~bq "\"type\"\\s*:\\s*\"premium\"""#).unwrap();
        let h = empty_headers();

        let ctx1 = make_ctx(
            "https://example.com/api/subscribe",
            "POST",
            &h,
            br#"{"type": "premium", "plan": "yearly"}"#,
            None,
            None,
            None,
        );
        assert!(f.matches(&ctx1));

        let ctx2 = make_ctx(
            "https://example.com/api/subscribe",
            "POST",
            &h,
            br#"{"type": "free", "plan": "monthly"}"#,
            None,
            None,
            None,
        );
        assert!(!f.matches(&ctx2));
    }
}
