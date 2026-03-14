use std::collections::{BTreeMap, HashMap};

/// URL 경로에서 파라미터를 추론합니다.
/// 예: /users/123 → /users/{id}, /posts/abc-def → /posts/{id}
pub fn infer_path_template(paths: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<Vec<String>>> = HashMap::new();

    for path in paths {
        let segments: Vec<String> = path
            .split('?')
            .next()
            .unwrap_or(path)
            .split('/')
            .map(|s| s.to_string())
            .collect();

        let key = segments.len().to_string();
        groups.entry(key).or_default().push(segments);
    }

    let mut templates: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (_key, segment_groups) in &groups {
        if segment_groups.is_empty() {
            continue;
        }

        let seg_len = segment_groups[0].len();
        let mut template_segments: Vec<String> = Vec::with_capacity(seg_len);

        for i in 0..seg_len {
            let values: Vec<&str> = segment_groups.iter().map(|s| s[i].as_str()).collect();
            let unique: std::collections::HashSet<&&str> = values.iter().collect();

            if unique.len() == 1 {
                template_segments.push(values[0].to_string());
            } else if values.iter().all(|v| is_dynamic_segment(v)) {
                let param_name = guess_param_name(i, &template_segments);
                template_segments.push(format!("{{{}}}", param_name));
            } else {
                // 일부만 다르면 그룹핑하지 않고 각각 유지
                for segs in segment_groups {
                    let path = segs.join("/");
                    if !path.is_empty() {
                        templates.entry(path).or_default();
                    }
                }
                template_segments.clear();
                break;
            }
        }

        if !template_segments.is_empty() {
            let template = template_segments.join("/");
            let original_paths: Vec<String> = segment_groups.iter().map(|s| s.join("/")).collect();
            templates.insert(template, original_paths);
        }
    }

    templates
}

/// 동적 세그먼트인지 판별 (숫자, UUID, 해시 등)
fn is_dynamic_segment(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }
    // 순수 숫자
    if segment.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    // UUID 패턴 (8-4-4-4-12)
    if segment.len() == 36 && segment.chars().filter(|c| *c == '-').count() == 4 {
        return segment.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    }
    // 긴 해시값 (16자 이상의 hex)
    if segment.len() >= 16 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    false
}

/// 파라미터 이름 추측
fn guess_param_name(index: usize, previous_segments: &[String]) -> String {
    if let Some(prev) = previous_segments.last() {
        let prev = prev.trim_end_matches('s'); // users → user
        if !prev.is_empty() && prev != "/" {
            return format!("{}Id", prev);
        }
    }
    format!(
        "id{}",
        if index > 0 {
            index.to_string()
        } else {
            String::new()
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dynamic_segment() {
        assert!(is_dynamic_segment("123"));
        assert!(is_dynamic_segment("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_dynamic_segment("abcdef0123456789"));
        assert!(!is_dynamic_segment("users"));
        assert!(!is_dynamic_segment("api"));
        assert!(!is_dynamic_segment(""));
    }

    #[test]
    fn test_infer_path_template() {
        let paths = vec![
            "/api/users/123".to_string(),
            "/api/users/456".to_string(),
            "/api/users/789".to_string(),
        ];
        let result = infer_path_template(&paths);
        assert!(result.contains_key("/api/users/{userId}"));
    }
}
