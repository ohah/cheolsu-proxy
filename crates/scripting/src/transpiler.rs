use crate::error::ScriptError;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use std::path::Path;

/// TypeScript 코드를 JavaScript로 트랜스파일 (oxc 사용)
pub fn transpile_ts(source: &str, filename: &str) -> Result<String, ScriptError> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();

    let parsed = Parser::new(&allocator, source, source_type).parse();

    if parsed.panicked {
        return Err(ScriptError::Transpile(
            "TypeScript 파싱 중 패닉 발생".to_string(),
        ));
    }

    if !parsed.errors.is_empty() {
        let errors: Vec<String> = parsed.errors.iter().map(|e| e.to_string()).collect();
        return Err(ScriptError::Transpile(format!(
            "TypeScript 파싱 오류: {}",
            errors.join(", ")
        )));
    }

    let mut program = parsed.program;

    let semantic_ret = SemanticBuilder::new()
        .with_excess_capacity(2.0)
        .build(&program);

    if !semantic_ret.errors.is_empty() {
        let errors: Vec<String> = semantic_ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(ScriptError::Transpile(format!(
            "시맨틱 분석 오류: {}",
            errors.join(", ")
        )));
    }

    let scoping = semantic_ret.semantic.into_scoping();

    let transform_options = TransformOptions::default();
    let ret = Transformer::new(&allocator, Path::new(filename), &transform_options)
        .build_with_scoping(scoping, &mut program);

    if !ret.errors.is_empty() {
        let errors: Vec<String> = ret.errors.iter().map(|e| e.to_string()).collect();
        return Err(ScriptError::Transpile(format!(
            "트랜스파일 오류: {}",
            errors.join(", ")
        )));
    }

    let js_code = Codegen::new().build(&program).code;
    Ok(js_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_typescript_transpile() {
        let ts_code = r#"
            const greeting: string = "hello";
            const num: number = 42;
            console.log(greeting, num);
        "#;
        let js = transpile_ts(ts_code, "test.ts").unwrap();
        assert!(js.contains("greeting"));
        assert!(js.contains("42"));
        assert!(!js.contains(": string"));
        assert!(!js.contains(": number"));
    }

    #[test]
    fn test_interface_removal() {
        let ts_code = r#"
            interface Request {
                method: string;
                url: string;
            }
            const req: Request = { method: "GET", url: "https://example.com" };
        "#;
        let js = transpile_ts(ts_code, "test.ts").unwrap();
        assert!(!js.contains("interface"));
        assert!(js.contains("req"));
    }

    #[test]
    fn test_function_with_types() {
        let ts_code = r#"
            function add(a: number, b: number): number {
                return a + b;
            }
        "#;
        let js = transpile_ts(ts_code, "test.ts").unwrap();
        assert!(js.contains("function add"));
        assert!(!js.contains(": number"));
    }
}
