// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod proxy_v2;
mod system_proxy;
use proxy_daemon::{ClientCommand, DaemonConnection, InterceptAction, InterceptRule};
use proxy_v2::{
    check_cli_installed, clean_old_proxy_cache, get_mcp_server_path, install_cli, load_script,
    proxy_v2_status, read_body_file, replay_request, replay_sequence, start_proxy_v2,
    stop_proxy_v2, uninstall_cli, unload_script, update_intercept_rules_v2, update_server_replay,
    update_upstream_proxy, ws_inject_message, ProxyV2State,
};
use system_proxy::get_proxy_status_command;
use tauri::Manager;
use tauri_plugin_cli::CliExt;

/// CLI 인터셉트 도움말 출력
fn print_cli_help() {
    println!();
    println!("명령어:");
    println!("  block <패턴> [옵션]                 요청 차단 (기본: 403)");
    println!("  modify-request <패턴> [옵션]        요청 수정 (별칭: mr)");
    println!("  modify-response <패턴> [옵션]       응답 수정 (별칭: mres)");
    println!("  rules                               현재 규칙 목록");
    println!("  remove <id>                         규칙 삭제");
    println!("  enable <id>                         규칙 활성화");
    println!("  disable <id>                        규칙 비활성화");
    println!("  clear                               모든 규칙 삭제");
    println!("  help                                도움말");
    println!();
    println!("패턴 (와일드카드):");
    println!("  *                                   모든 문자열 매칭");
    println!("  ?                                   단일 문자 매칭");
    println!();
    println!("옵션:");
    println!("  --header <name>=<value>             헤더 추가 (여러 개 가능)");
    println!("  --remove-header <name>              헤더 제거 (여러 개 가능)");
    println!("  --body <body>                       바디 설정");
    println!("  --status <code>                     상태 코드 설정");
    println!("  --method <METHOD>                   특정 HTTP 메서드만 매칭");
    println!("  --name <name>                       규칙 이름 설정");
    println!();
    println!("예시:");
    println!("  > block *ads.example.com*");
    println!("  > block *.example.com/api/* --status 403");
    println!("  > mres *example.com/api* --body '{{\"mocked\":true}}' --status 200");
    println!("  > mr *example.com* --header X-Debug=true --method GET");
    println!();
}

/// CLI 명령어를 파싱하여 인터셉트 규칙으로 변환
fn parse_cli_command(
    input: &str,
    rules: &mut Vec<InterceptRule>,
    rule_counter: &mut u32,
) -> Option<Vec<InterceptRule>> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // 토큰 분리 (따옴표 내 공백 보존)
    let tokens = shell_split(input);
    if tokens.is_empty() {
        return None;
    }

    let cmd = tokens[0].to_lowercase();
    match cmd.as_str() {
        "help" | "h" | "?" => {
            print_cli_help();
            None
        }
        "rules" | "list" | "ls" => {
            if rules.is_empty() {
                println!("  (규칙 없음)");
            } else {
                for rule in rules.iter() {
                    println!("  {}", rule);
                }
            }
            None
        }
        "remove" | "rm" | "delete" => {
            if tokens.len() < 2 {
                println!("  사용법: remove <id>");
                return None;
            }
            let id = &tokens[1];
            let before = rules.len();
            rules.retain(|r| r.id != *id);
            if rules.len() < before {
                println!("  규칙 삭제됨: {}", id);
                Some(rules.clone())
            } else {
                println!("  규칙을 찾을 수 없음: {}", id);
                None
            }
        }
        "enable" => {
            if tokens.len() < 2 {
                println!("  사용법: enable <id>");
                return None;
            }
            let id = &tokens[1];
            if let Some(rule) = rules.iter_mut().find(|r| r.id == *id) {
                rule.enabled = true;
                println!("  규칙 활성화: {}", rule);
                Some(rules.clone())
            } else {
                println!("  규칙을 찾을 수 없음: {}", id);
                None
            }
        }
        "disable" => {
            if tokens.len() < 2 {
                println!("  사용법: disable <id>");
                return None;
            }
            let id = &tokens[1];
            if let Some(rule) = rules.iter_mut().find(|r| r.id == *id) {
                rule.enabled = false;
                println!("  규칙 비활성화: {}", rule);
                Some(rules.clone())
            } else {
                println!("  규칙을 찾을 수 없음: {}", id);
                None
            }
        }
        "clear" => {
            rules.clear();
            println!("  모든 규칙 삭제됨");
            Some(rules.clone())
        }
        "block" => {
            if tokens.len() < 2 {
                println!("  사용법: block <패턴> [옵션]");
                println!("  예시: block *ads.example.com* --status 403 --body Blocked");
                return None;
            }
            let pattern = tokens[1].clone();
            let opts = parse_common_options(&tokens[2..]);
            let status_code = opts.status.unwrap_or(403);
            let body = opts.body.unwrap_or_default();

            *rule_counter += 1;
            let id = format!("r{}", rule_counter);
            let name = opts.name.unwrap_or_else(|| format!("Block {}", pattern));

            let rule = InterceptRule {
                id: id.clone(),
                name,
                enabled: true,
                pattern,
                method: opts.method,
                action: InterceptAction::Block { status_code, body },
            };
            println!("  규칙 추가: {}", rule);
            rules.push(rule);
            Some(rules.clone())
        }
        "modify-request" | "mr" => {
            if tokens.len() < 2 {
                println!("  사용법: modify-request <패턴> [옵션]");
                println!("  예시: mr *api.com* --header X-Auth=token --method POST");
                return None;
            }
            let pattern = tokens[1].clone();
            let opts = parse_common_options(&tokens[2..]);

            *rule_counter += 1;
            let id = format!("r{}", rule_counter);
            let name = opts
                .name
                .unwrap_or_else(|| format!("ModifyReq {}", pattern));

            let rule = InterceptRule {
                id: id.clone(),
                name,
                enabled: true,
                pattern,
                method: opts.method,
                action: InterceptAction::ModifyRequest {
                    add_headers: opts.add_headers,
                    remove_headers: opts.remove_headers,
                    set_body: opts.body,
                },
            };
            println!("  규칙 추가: {}", rule);
            rules.push(rule);
            Some(rules.clone())
        }
        "modify-response" | "mres" => {
            if tokens.len() < 2 {
                println!("  사용법: modify-response <패턴> [옵션]");
                println!("  예시: mres *api.com* --status 200 --body '{{\"ok\":true}}'");
                return None;
            }
            let pattern = tokens[1].clone();
            let opts = parse_common_options(&tokens[2..]);

            *rule_counter += 1;
            let id = format!("r{}", rule_counter);
            let name = opts
                .name
                .unwrap_or_else(|| format!("ModifyRes {}", pattern));

            let rule = InterceptRule {
                id: id.clone(),
                name,
                enabled: true,
                pattern,
                method: opts.method,
                action: InterceptAction::ModifyResponse {
                    set_status: opts.status,
                    add_headers: opts.add_headers,
                    remove_headers: opts.remove_headers,
                    set_body: opts.body,
                },
            };
            println!("  규칙 추가: {}", rule);
            rules.push(rule);
            Some(rules.clone())
        }
        _ => {
            println!("  알 수 없는 명령어: {}. 'help'를 입력하세요.", cmd);
            None
        }
    }
}

/// 공통 CLI 옵션
struct CliOptions {
    add_headers: std::collections::HashMap<String, String>,
    remove_headers: Vec<String>,
    body: Option<String>,
    status: Option<u16>,
    method: Option<String>,
    name: Option<String>,
}

/// 공통 옵션 파싱
fn parse_common_options(tokens: &[String]) -> CliOptions {
    let mut opts = CliOptions {
        add_headers: std::collections::HashMap::new(),
        remove_headers: Vec::new(),
        body: None,
        status: None,
        method: None,
        name: None,
    };

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "--header" | "-H" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    if let Some((k, v)) = tokens[i].split_once('=') {
                        opts.add_headers.insert(k.to_string(), v.to_string());
                    }
                }
            }
            "--remove-header" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    opts.remove_headers.push(tokens[i].clone());
                }
            }
            "--body" | "-b" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    opts.body = Some(tokens[i].clone());
                }
            }
            "--status" | "-s" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    opts.status = tokens[i].parse().ok();
                }
            }
            "--method" | "-m" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    opts.method = Some(tokens[i].to_uppercase());
                }
            }
            "--name" | "-n" => {
                if i + 1 < tokens.len() {
                    i += 1;
                    opts.name = Some(tokens[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    opts
}

/// 셸 스타일 문자열 분리 (따옴표 내 공백 보존)
fn shell_split(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }
        match ch {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// CLI에서 STDIN 명령을 읽고 인터셉트 규칙을 관리하는 태스크
async fn run_cli_intercept_loop(conn: &DaemonConnection) {
    let mut rules: Vec<InterceptRule> = Vec::new();
    let mut rule_counter: u32 = 0;

    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = tokio::io::AsyncBufReadExt::lines(reader);

    print!("> ");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if let Some(updated_rules) = parse_cli_command(&line, &mut rules, &mut rule_counter)
                {
                    let cmd = ClientCommand::UpdateInterceptRules {
                        rules: updated_rules,
                    };
                    if let Err(e) = conn.send_command(&cmd).await {
                        eprintln!("  규칙 전송 실패: {}", e);
                    }
                }
                print!("> ");
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            Ok(None) => break,
            Err(e) => {
                eprintln!("STDIN 읽기 오류: {}", e);
                break;
            }
        }
    }
}

/// headless (CLI) 모드인지 확인하고, headless일 경우 daemon 클라이언트로 동작합니다.
fn handle_cli_mode(app: &tauri::App) -> bool {
    let cli_matches = match app.cli().matches() {
        Ok(m) => m,
        Err(_) => return false,
    };

    let headless = cli_matches
        .args
        .get("headless")
        .map(|v| v.occurrences > 0)
        .unwrap_or(false);

    if !headless {
        return false;
    }

    // verbose 모드
    let verbose = cli_matches
        .args
        .get("verbose")
        .map(|v| v.occurrences > 0)
        .unwrap_or(false);

    // 포트 파싱 (기본값 8080)
    let port: u16 = cli_matches
        .args
        .get("port")
        .and_then(|v| v.value.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // 호스트 파싱 (기본값 127.0.0.1)
    let host = cli_matches
        .args
        .get("host")
        .and_then(|v| v.value.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    println!("========================================");
    println!(" Cheolsu Proxy — Headless Mode");
    println!("========================================");
    println!("  Listen : {}:{}", host, port);
    println!("  Verbose: {}", verbose);
    println!("========================================");
    println!();
    println!("'help'를 입력하면 인터셉트 명령어를 확인할 수 있습니다.");

    // GUI 윈도우 숨기기 (close하면 Tauri가 앱을 종료함)
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    // headless 모드: daemon에 연결하여 이벤트를 stdout에 출력
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        match proxy_daemon::ensure_daemon(port, &host, move |msg| {
            if verbose {
                if let proxy_daemon::DaemonMessage::Event { data: ref event } = msg {
                    // verbose 모드에서는 요청/응답 정보를 stdout에 출력
                    if let Some(req) = &event.0 {
                        println!(
                            "  [REQ] {} {} ({:?})",
                            req.method(),
                            req.uri(),
                            req.data_type()
                        );
                    }
                    if let Some(res) = &event.1 {
                        println!("  [RES] {} ({:?})", res.status(), res.data_type());
                    }
                }
            }
        })
        .await
        {
            Ok(conn) => {
                println!("Connected to daemon (port {})", conn.port);
                println!();

                // CLI 인터셉트 루프와 Ctrl+C를 동시에 대기
                tokio::select! {
                    _ = run_cli_intercept_loop(&conn) => {
                        println!("\nSTDIN 종료됨");
                        conn.disconnect().await;
                        app_handle.exit(0);
                    }
                    _ = tokio::signal::ctrl_c() => {
                        println!("\nShutting down...");
                        conn.disconnect().await;
                        app_handle.exit(0);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to daemon: {}", e);
                app_handle.exit(1);
            }
        }
    });

    true
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    // let devtools = tauri_plugin_devtools::init();
    {
        let mut builder = tauri::Builder::default()
            .plugin(tauri_plugin_cli::init())
            .plugin(tauri_plugin_http::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_clipboard_manager::init())
            .plugin(tauri_plugin_store::Builder::default().build());

        #[cfg(debug_assertions)]
        {
            builder = builder.plugin(tauri_plugin_mcp_bridge::init());
        }

        // DevTools 플러그인 추가 (개발 빌드에서만)
        // #[cfg(debug_assertions)]
        // {
        //     builder = builder.plugin(devtools);
        // }

        builder
            .setup(|app_handle| {
                // proxyapi_v2 프록시 상태
                app_handle.manage(ProxyV2State::default());

                // CLI 모드 확인 — headless이면 GUI 셋업 건너뜀
                if handle_cli_mode(app_handle) {
                    return Ok(());
                }

                // 앱 시작 시 자동 캐시 정리 (1일 이상 된 캐시)
                tauri::async_runtime::spawn(async {
                    match clean_old_proxy_cache(1).await {
                        Ok(message) => println!("{}", message),
                        Err(e) => eprintln!("캐시 정리 실패: {}", e),
                    }
                });

                // GUI 모드에서는 시스템 프록시 설정을 daemon이 담당하므로 여기서는 하지 않음

                Ok(())
            })
            .on_window_event(|_window, event| {
                // GUI 종료 시 daemon과의 연결만 해제 (프록시 설정 해제는 daemon이 담당)
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    println!("CloseRequested");
                    // daemon과의 연결은 ProxyV2State drop 시 자동 해제
                }
            })
            .invoke_handler(tauri::generate_handler![
                start_proxy_v2,
                stop_proxy_v2,
                proxy_v2_status,
                update_intercept_rules_v2,
                get_proxy_status_command,
                read_body_file,
                clean_old_proxy_cache,
                replay_request,
                replay_sequence,
                ws_inject_message,
                update_upstream_proxy,
                update_server_replay,
                get_mcp_server_path,
                install_cli,
                uninstall_cli,
                check_cli_installed,
                load_script,
                unload_script
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
