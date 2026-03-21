use cheolsu_ops::result::OpResult;
use clap::ValueEnum;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

pub fn print_result(result: OpResult, format: OutputFormat) -> i32 {
    match (&result, format) {
        (OpResult::Ok(msg), OutputFormat::Json) => {
            let json = serde_json::json!({ "status": "ok", "message": msg });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
            0
        }
        (OpResult::Err(msg), OutputFormat::Json) => {
            let json = serde_json::json!({ "status": "error", "message": msg });
            eprintln!("{}", serde_json::to_string_pretty(&json).unwrap());
            1
        }
        (OpResult::Ok(msg), OutputFormat::Text) => {
            println!("{}", msg);
            0
        }
        (OpResult::Err(msg), OutputFormat::Text) => {
            eprintln!("Error: {}", msg);
            1
        }
    }
}
