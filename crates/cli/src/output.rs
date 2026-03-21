use cheolsu_ops::result::OpResult;

/// OpResult를 stdout/stderr로 출력하고 exit code를 반환한다.
pub fn print_result(result: OpResult) -> i32 {
    match result {
        OpResult::Ok(msg) => {
            println!("{}", msg);
            0
        }
        OpResult::Err(msg) => {
            eprintln!("Error: {}", msg);
            1
        }
    }
}
