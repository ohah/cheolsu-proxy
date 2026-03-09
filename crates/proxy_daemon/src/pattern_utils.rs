use regex::Regex;

/// 와일드카드 패턴 매칭 (* = 임의 문자열, ? = 단일 문자)
pub(crate) fn wildcard_matches(pattern: &str, text: &str) -> bool {
    let regex_pattern = format!(
        "(?i){}",
        regex::escape(pattern)
            .replace("\\*", ".*")
            .replace("\\?", ".")
    );
    Regex::new(&regex_pattern)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_star_prefix() {
        assert!(wildcard_matches(
            "*ads.example.com*",
            "https://ads.example.com/banner"
        ));
    }

    #[test]
    fn test_wildcard_subdomain_and_path() {
        assert!(wildcard_matches(
            "*.example.com/api/*",
            "https://sub.example.com/api/v1/users"
        ));
    }

    #[test]
    fn test_wildcard_no_match() {
        assert!(!wildcard_matches(
            "*ads.example.com*",
            "https://other.com/page"
        ));
    }

    #[test]
    fn test_wildcard_exact_domain() {
        assert!(wildcard_matches("*example.com*", "https://example.com"));
    }

    #[test]
    fn test_wildcard_path_only() {
        assert!(wildcard_matches(
            "*/api/v1/*",
            "https://any.com/api/v1/users"
        ));
        assert!(!wildcard_matches(
            "*/api/v1/*",
            "https://any.com/api/v2/users"
        ));
    }

    #[test]
    fn test_wildcard_question_mark() {
        assert!(wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v1/users"
        ));
        assert!(wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v2/users"
        ));
        assert!(!wildcard_matches(
            "*api/v?/users*",
            "https://example.com/api/v10/users"
        ));
    }

    #[test]
    fn test_wildcard_case_insensitive() {
        assert!(wildcard_matches(
            "*Example.COM*",
            "https://example.com/page"
        ));
    }

    #[test]
    fn test_wildcard_catch_all() {
        assert!(wildcard_matches("*", "https://anything.com/any/path"));
    }

    #[test]
    fn test_wildcard_no_wildcards_partial() {
        assert!(wildcard_matches("example.com", "https://example.com/api"));
    }

    #[test]
    fn test_wildcard_special_chars_escaped() {
        assert!(wildcard_matches(
            "*example.com/api?key=*",
            "https://example.com/api?key=value"
        ));
    }
}
