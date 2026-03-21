mod connection;
mod output;

use cheolsu_ops::params::*;
use clap::{Args, Parser, Subcommand};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "cheolsu", about = "Cheolsu Proxy CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 트래픽 검색 및 조회
    #[command(subcommand)]
    Traffic(TrafficCommands),

    /// 인터셉트 규칙 관리
    #[command(subcommand)]
    Rule(RuleCommands),

    /// 브레이크포인트 관리
    #[command(subcommand)]
    Breakpoint(BreakpointCommands),

    /// 호스트 매핑 관리
    #[command(subcommand)]
    HostMapping(HostMappingCommands),

    /// 리버스 프록시 규칙 관리
    #[command(subcommand)]
    ReverseProxy(ReverseProxyCommands),

    /// 서버 리플레이 관리
    #[command(subcommand)]
    ServerReplay(ServerReplayCommands),

    /// 프록시 설정 변경
    #[command(subcommand)]
    Settings(SettingsCommands),

    /// 트래픽 분석
    #[command(subcommand)]
    Analyze(AnalyzeCommands),

    /// HTTP 요청 전송
    #[command(subcommand)]
    Replay(ReplayCommands),

    /// 세션 관리
    #[command(subcommand)]
    Session(SessionCommands),

    /// 스크립트 관리
    #[command(subcommand)]
    Script(ScriptCommands),

    /// 프록시 상태 확인
    Status,

    /// HAR 파일로 내보내기
    ExportHar(ExportHarArgs),

    /// TUI 모드로 실행
    Tui(TuiArgs),
}

// ─── TUI ─────────────────────────────────────────────────

#[derive(Args)]
struct TuiArgs {
    /// 프록시 포트
    #[arg(short, long, default_value_t = 8100)]
    port: u16,
    /// 바인드 호스트
    #[arg(short = 'b', long, default_value = "0.0.0.0")]
    host: String,
}

// ─── Traffic ─────────────────────────────────────────────

#[derive(Subcommand)]
enum TrafficCommands {
    /// 캡처된 트래픽 검색
    Search(SearchTrafficArgs),
    /// 트랜잭션 상세 조회
    Get { id: String },
    /// WebSocket 메시지 조회
    Ws(WsArgs),
    /// SSE 이벤트 조회
    Sse(SseArgs),
    /// 두 트랜잭션 비교
    Diff {
        /// 첫 번째 트랜잭션 ID
        id_a: String,
        /// 두 번째 트랜잭션 ID
        id_b: String,
    },
    /// 캡처된 트래픽 전체 삭제
    Clear,
    /// OpenAPI 스펙 생성
    Openapi(OpenapiArgs),
}

#[derive(Args)]
struct SearchTrafficArgs {
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    method: Option<String>,
    #[arg(long)]
    status: Option<u16>,
    #[arg(long)]
    path: Option<String>,
    #[arg(short, long, default_value_t = 50)]
    limit: usize,
}

#[derive(Args)]
struct WsArgs {
    #[arg(long)]
    connection_id: Option<String>,
    #[arg(short, long, default_value_t = 100)]
    limit: usize,
}

#[derive(Args)]
struct SseArgs {
    #[arg(long)]
    connection_id: Option<String>,
    #[arg(short, long, default_value_t = 100)]
    limit: usize,
}

#[derive(Args)]
struct OpenapiArgs {
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    path_prefix: Option<String>,
    #[arg(long)]
    title: Option<String>,
}

// ─── Rules ───────────────────────────────────────────────

#[derive(Subcommand)]
enum RuleCommands {
    /// 규칙 목록 조회
    List,
    /// 규칙 추가
    Add(AddRuleArgs),
    /// 규칙 제거
    Remove { id: String },
}

#[derive(Args)]
struct AddRuleArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    pattern: String,
    #[arg(long)]
    method: Option<String>,
    #[arg(long)]
    action_type: String,
    #[arg(long)]
    status_code: Option<u16>,
    #[arg(long)]
    response_body: Option<String>,
    #[arg(long, value_parser = parse_key_val)]
    add_header: Vec<(String, String)>,
    #[arg(long)]
    remove_header: Vec<String>,
    #[arg(long)]
    file_path: Option<String>,
    #[arg(long)]
    target_url: Option<String>,
    #[arg(long)]
    preserve_path: Option<bool>,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("'key=value' 형식이 아닙니다: {}", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

// ─── Breakpoints ─────────────────────────────────────────

#[derive(Subcommand)]
enum BreakpointCommands {
    List,
    Add(AddBreakpointArgs),
    Remove { id: String },
    Pending,
    Resolve(ResolveBreakpointArgs),
}

#[derive(Args)]
struct AddBreakpointArgs {
    #[arg(long)]
    pattern: String,
    #[arg(long)]
    break_on_request: Option<bool>,
    #[arg(long)]
    break_on_response: Option<bool>,
}

#[derive(Args)]
struct ResolveBreakpointArgs {
    #[arg(long)]
    id: String,
    #[arg(long)]
    action: String,
    #[arg(long, value_parser = parse_key_val)]
    header: Vec<(String, String)>,
    #[arg(long)]
    body: Option<String>,
    #[arg(long)]
    status: Option<u16>,
}

// ─── Host Mapping ────────────────────────────────────────

#[derive(Subcommand)]
enum HostMappingCommands {
    List,
    Add(AddHostMappingArgs),
    Remove { id: String },
}

#[derive(Args)]
struct AddHostMappingArgs {
    #[arg(long)]
    source_host: String,
    #[arg(long)]
    source_port: Option<u16>,
    #[arg(long)]
    target_host: String,
    #[arg(long)]
    target_port: Option<u16>,
}

// ─── Reverse Proxy ───────────────────────────────────────

#[derive(Subcommand)]
enum ReverseProxyCommands {
    List,
    Add(AddReverseProxyArgs),
    Remove { id: String },
}

#[derive(Args)]
struct AddReverseProxyArgs {
    #[arg(long)]
    match_host: String,
    #[arg(long)]
    backend_scheme: Option<String>,
    #[arg(long)]
    backend_host: String,
    #[arg(long)]
    backend_port: u16,
    #[arg(long)]
    rewrite_host: Option<bool>,
}

// ─── Server Replay ───────────────────────────────────────

#[derive(Subcommand)]
enum ServerReplayCommands {
    List,
    Add { transaction_id: String },
    Remove { id: String },
    Clear,
}

// ─── Settings ────────────────────────────────────────────

#[derive(Subcommand)]
enum SettingsCommands {
    /// 업스트림 프록시 설정
    UpstreamProxy(UpstreamProxyArgs),
    /// 네트워크 쓰로틀링 설정
    Throttle(ThrottleArgs),
    /// 프록시 인증 설정
    ProxyAuth(ProxyAuthArgs),
    /// 연결 전략 설정
    ConnectionStrategy { strategy: String },
    /// 빠른 설정
    Quick(QuickSettingsArgs),
}

#[derive(Args)]
struct UpstreamProxyArgs {
    #[arg(long)]
    enabled: bool,
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    username: Option<String>,
    #[arg(long)]
    password: Option<String>,
}

#[derive(Args)]
struct ThrottleArgs {
    #[arg(long)]
    enabled: bool,
    #[arg(long)]
    download_rate: Option<u64>,
    #[arg(long)]
    upload_rate: Option<u64>,
    #[arg(long)]
    latency_ms: Option<u64>,
}

#[derive(Args)]
struct ProxyAuthArgs {
    #[arg(long)]
    enabled: bool,
    #[arg(long)]
    method: Option<String>,
    #[arg(long)]
    username: Option<String>,
    #[arg(long)]
    password: Option<String>,
    #[arg(long)]
    token: Option<String>,
}

#[derive(Args)]
struct QuickSettingsArgs {
    #[arg(long)]
    no_caching: bool,
    #[arg(long)]
    block_cookies: bool,
    #[arg(long)]
    no_gzip: bool,
}

// ─── Analyze ─────────────────────────────────────────────

#[derive(Subcommand)]
enum AnalyzeCommands {
    /// 느린 요청 분석
    Performance {
        #[arg(long, default_value_t = 1000)]
        threshold_ms: u64,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// 에러 분석
    Errors,
    /// 엔드포인트별 통계
    Endpoints {
        #[arg(long)]
        sort_by: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: usize,
    },
    /// 중복 요청 탐지
    Duplicates {
        #[arg(long, default_value_t = 3000)]
        window_ms: u64,
    },
    /// N+1 패턴 탐지
    NPlus1,
    /// 트래픽 타임라인
    Timeline {
        #[arg(long, default_value_t = 60)]
        bucket_seconds: u64,
    },
    /// 전체 분석
    Full,
}

// ─── Replay ──────────────────────────────────────────────

#[derive(Subcommand)]
enum ReplayCommands {
    /// HTTP 요청 직접 전송
    Request(ReplayRequestArgs),
    /// 여러 트랜잭션 순차 재전송
    Sequence(ReplaySequenceArgs),
    /// 반복 요청 (부하 테스트)
    Repeat(RepeatArgs),
}

#[derive(Args)]
struct ReplayRequestArgs {
    #[arg(long)]
    method: String,
    #[arg(long)]
    url: String,
    #[arg(long, value_parser = parse_key_val)]
    header: Vec<(String, String)>,
    #[arg(long)]
    body: Option<String>,
}

#[derive(Args)]
struct ReplaySequenceArgs {
    /// 트랜잭션 ID 목록
    ids: Vec<String>,
    #[arg(long, default_value_t = 0)]
    delay_ms: u64,
}

#[derive(Args)]
struct RepeatArgs {
    #[arg(long)]
    method: String,
    #[arg(long)]
    url: String,
    #[arg(long, value_parser = parse_key_val)]
    header: Vec<(String, String)>,
    #[arg(long)]
    body: Option<String>,
    #[arg(long, default_value_t = 10)]
    iterations: u32,
    #[arg(long, default_value_t = 1)]
    concurrency: u32,
    #[arg(long, default_value_t = 0)]
    delay_ms: u64,
}

// ─── Session ─────────────────────────────────────────────

#[derive(Subcommand)]
enum SessionCommands {
    /// 세션 저장
    Save(SaveSessionArgs),
    /// 세션 로드
    Load(LoadSessionArgs),
}

#[derive(Args)]
struct SaveSessionArgs {
    path: String,
    #[arg(long)]
    filter: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    description: Option<String>,
}

#[derive(Args)]
struct LoadSessionArgs {
    path: String,
    #[arg(long)]
    append: bool,
}

// ─── Script ──────────────────────────────────────────────

#[derive(Subcommand)]
enum ScriptCommands {
    /// 스크립트 로드
    Load {
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        code: Option<String>,
    },
    /// 스크립트 언로드
    Unload,
}

// ─── Export HAR ──────────────────────────────────────────

#[derive(Args)]
struct ExportHarArgs {
    output_path: String,
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    path: Option<String>,
}

// ─── TUI exec ────────────────────────────────────────────

fn exec_tui(args: &TuiArgs) -> i32 {
    let mut cmd = std::process::Command::new("cheolsu-tui");
    cmd.arg("--port").arg(args.port.to_string());
    cmd.arg("--host").arg(&args.host);
    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!(
                "Error: cheolsu-tui를 실행할 수 없습니다: {}\n\
                 cheolsu-tui가 PATH에 설치되어 있는지 확인해주세요.",
                e
            );
            1
        }
    }
}

// ─── Main ────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let exit_code = run(cli).await;
    std::process::exit(exit_code);
}

async fn run(cli: Cli) -> i32 {
    // tui 서브커맨드: cheolsu-tui 바이너리를 exec
    if let Commands::Tui(args) = &cli.command {
        return exec_tui(args);
    }

    // replay request/repeat는 daemon 연결 불필요
    if let Commands::Replay(ReplayCommands::Request(args)) = &cli.command {
        let headers = if args.header.is_empty() {
            None
        } else {
            Some(args.header.iter().cloned().collect::<HashMap<_, _>>())
        };
        let result = cheolsu_ops::replay::replay_request(ReplayRequestParams {
            method: args.method.clone(),
            url: args.url.clone(),
            headers,
            body: args.body.clone(),
        })
        .await;
        return output::print_result(result);
    }
    if let Commands::Replay(ReplayCommands::Repeat(args)) = &cli.command {
        let headers = if args.header.is_empty() {
            None
        } else {
            Some(args.header.iter().cloned().collect::<HashMap<_, _>>())
        };
        let result = cheolsu_ops::replay::advanced_repeat(AdvancedRepeatParams {
            method: args.method.clone(),
            url: args.url.clone(),
            headers,
            body: args.body.clone(),
            iterations: Some(args.iterations),
            concurrency: Some(args.concurrency),
            delay_ms: Some(args.delay_ms),
        })
        .await;
        return output::print_result(result);
    }

    // 나머지 커맨드는 daemon 연결 필요
    let ctx = match connection::connect_and_sync().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    let result = match cli.command {
        Commands::Status => cheolsu_ops::traffic::proxy_status(&ctx).await,

        // Traffic
        Commands::Traffic(cmd) => match cmd {
            TrafficCommands::Search(args) => cheolsu_ops::traffic::search_traffic(
                &ctx,
                SearchTrafficParams {
                    host: args.host,
                    method: args.method,
                    status: args.status,
                    path: args.path,
                    limit: Some(args.limit),
                },
            ),
            TrafficCommands::Get { id } => {
                cheolsu_ops::traffic::get_transaction(&ctx, GetTransactionParams { id })
            }
            TrafficCommands::Ws(args) => cheolsu_ops::traffic::get_websocket_messages(
                &ctx,
                GetWsMessagesParams {
                    connection_id: args.connection_id,
                    limit: Some(args.limit),
                },
            ),
            TrafficCommands::Sse(args) => cheolsu_ops::sse::get_sse_events(
                &ctx,
                GetSseEventsParams {
                    connection_id: args.connection_id,
                    limit: Some(args.limit),
                },
            ),
            TrafficCommands::Diff { id_a, id_b } => cheolsu_ops::traffic::diff_transactions(
                &ctx,
                DiffTransactionsParams {
                    transaction_id_a: id_a,
                    transaction_id_b: id_b,
                },
            ),
            TrafficCommands::Clear => cheolsu_ops::traffic::clear_traffic(&ctx),
            TrafficCommands::Openapi(args) => cheolsu_ops::traffic::generate_openapi_spec(
                &ctx,
                GenerateOpenApiParams {
                    host: args.host,
                    path_prefix: args.path_prefix,
                    format: None,
                    title: args.title,
                },
            ),
        },

        // Rules
        Commands::Rule(cmd) => match cmd {
            RuleCommands::List => cheolsu_ops::rules::list_rules(&ctx),
            RuleCommands::Add(args) => {
                cheolsu_ops::rules::add_rule(
                    &ctx,
                    AddRuleParams {
                        name: args.name,
                        pattern: args.pattern,
                        method: args.method,
                        action_type: args.action_type,
                        status_code: args.status_code,
                        response_body: args.response_body,
                        add_headers: if args.add_header.is_empty() {
                            None
                        } else {
                            Some(args.add_header.into_iter().collect())
                        },
                        remove_headers: if args.remove_header.is_empty() {
                            None
                        } else {
                            Some(args.remove_header)
                        },
                        file_path: args.file_path,
                        target_url: args.target_url,
                        preserve_path: args.preserve_path,
                    },
                )
                .await
            }
            RuleCommands::Remove { id } => {
                cheolsu_ops::rules::remove_rule(&ctx, RemoveRuleParams { id }).await
            }
        },

        // Breakpoints
        Commands::Breakpoint(cmd) => match cmd {
            BreakpointCommands::List => cheolsu_ops::breakpoints::list_breakpoints(&ctx),
            BreakpointCommands::Add(args) => {
                cheolsu_ops::breakpoints::add_breakpoint(
                    &ctx,
                    AddBreakpointParams {
                        pattern: args.pattern,
                        break_on_request: args.break_on_request,
                        break_on_response: args.break_on_response,
                    },
                )
                .await
            }
            BreakpointCommands::Remove { id } => {
                cheolsu_ops::breakpoints::remove_breakpoint(&ctx, RemoveBreakpointParams { id })
                    .await
            }
            BreakpointCommands::Pending => cheolsu_ops::breakpoints::list_pending_breakpoints(),
            BreakpointCommands::Resolve(args) => {
                cheolsu_ops::breakpoints::resolve_breakpoint(
                    &ctx,
                    ResolveBreakpointParams {
                        id: args.id,
                        action: args.action,
                        headers: if args.header.is_empty() {
                            None
                        } else {
                            Some(args.header.into_iter().collect())
                        },
                        body: args.body,
                        status: args.status,
                    },
                )
                .await
            }
        },

        // Host Mapping
        Commands::HostMapping(cmd) => match cmd {
            HostMappingCommands::List => cheolsu_ops::host_mappings::list_host_mappings(&ctx),
            HostMappingCommands::Add(args) => {
                cheolsu_ops::host_mappings::add_host_mapping(
                    &ctx,
                    AddHostMappingParams {
                        source_host: args.source_host,
                        source_port: args.source_port,
                        target_host: args.target_host,
                        target_port: args.target_port,
                    },
                )
                .await
            }
            HostMappingCommands::Remove { id } => {
                cheolsu_ops::host_mappings::remove_host_mapping(
                    &ctx,
                    RemoveHostMappingParams { id },
                )
                .await
            }
        },

        // Reverse Proxy
        Commands::ReverseProxy(cmd) => match cmd {
            ReverseProxyCommands::List => {
                cheolsu_ops::reverse_proxy::list_reverse_proxy_rules(&ctx)
            }
            ReverseProxyCommands::Add(args) => {
                cheolsu_ops::reverse_proxy::add_reverse_proxy_rule(
                    &ctx,
                    AddReverseProxyRuleParams {
                        match_host: args.match_host,
                        backend_scheme: args.backend_scheme,
                        backend_host: args.backend_host,
                        backend_port: args.backend_port,
                        rewrite_host: args.rewrite_host,
                    },
                )
                .await
            }
            ReverseProxyCommands::Remove { id } => {
                cheolsu_ops::reverse_proxy::remove_reverse_proxy_rule(
                    &ctx,
                    RemoveReverseProxyRuleParams { id },
                )
                .await
            }
        },

        // Server Replay
        Commands::ServerReplay(cmd) => match cmd {
            ServerReplayCommands::List => cheolsu_ops::server_replay::list_server_replay(&ctx),
            ServerReplayCommands::Add { transaction_id } => {
                cheolsu_ops::server_replay::add_server_replay(
                    &ctx,
                    AddServerReplayParams { transaction_id },
                )
                .await
            }
            ServerReplayCommands::Remove { id } => {
                cheolsu_ops::server_replay::remove_server_replay(
                    &ctx,
                    RemoveServerReplayParams { id },
                )
                .await
            }
            ServerReplayCommands::Clear => {
                cheolsu_ops::server_replay::clear_server_replay(&ctx).await
            }
        },

        // Settings
        Commands::Settings(cmd) => match cmd {
            SettingsCommands::UpstreamProxy(args) => {
                cheolsu_ops::settings::update_upstream_proxy(
                    &ctx,
                    UpdateUpstreamProxyParams {
                        enabled: args.enabled,
                        host: args.host,
                        port: args.port,
                        username: args.username,
                        password: args.password,
                        bypass: None,
                    },
                )
                .await
            }
            SettingsCommands::Throttle(args) => {
                cheolsu_ops::settings::update_throttle(
                    &ctx,
                    UpdateThrottleParams {
                        enabled: args.enabled,
                        download_rate: args.download_rate,
                        upload_rate: args.upload_rate,
                        latency_ms: args.latency_ms,
                    },
                )
                .await
            }
            SettingsCommands::ProxyAuth(args) => {
                cheolsu_ops::settings::update_proxy_auth(
                    &ctx,
                    UpdateProxyAuthParams {
                        enabled: args.enabled,
                        method: args.method,
                        username: args.username,
                        password: args.password,
                        token: args.token,
                    },
                )
                .await
            }
            SettingsCommands::ConnectionStrategy { strategy } => {
                cheolsu_ops::settings::update_connection_strategy(
                    &ctx,
                    UpdateConnectionStrategyParams { strategy },
                )
                .await
            }
            SettingsCommands::Quick(args) => {
                cheolsu_ops::settings::update_quick_settings(
                    &ctx,
                    UpdateQuickSettingsParams {
                        no_caching: args.no_caching,
                        block_cookies: args.block_cookies,
                        no_gzip: args.no_gzip,
                    },
                )
                .await
            }
        },

        // Analyze
        Commands::Analyze(cmd) => match cmd {
            AnalyzeCommands::Performance {
                threshold_ms,
                limit,
            } => cheolsu_ops::analyze::analyze_performance(
                &ctx,
                AnalyzePerformanceParams {
                    threshold_ms: Some(threshold_ms),
                    limit: Some(limit),
                },
            ),
            AnalyzeCommands::Errors => cheolsu_ops::analyze::analyze_errors(&ctx),
            AnalyzeCommands::Endpoints { sort_by, limit } => {
                cheolsu_ops::analyze::analyze_endpoints(
                    &ctx,
                    AnalyzeEndpointsParams {
                        sort_by,
                        limit: Some(limit),
                    },
                )
            }
            AnalyzeCommands::Duplicates { window_ms } => cheolsu_ops::analyze::detect_duplicates(
                &ctx,
                DetectDuplicatesParams {
                    window_ms: Some(window_ms),
                },
            ),
            AnalyzeCommands::NPlus1 => cheolsu_ops::analyze::detect_n_plus_one(&ctx),
            AnalyzeCommands::Timeline { bucket_seconds } => {
                cheolsu_ops::analyze::analyze_traffic_timeline(
                    &ctx,
                    AnalyzeTimelineParams {
                        bucket_seconds: Some(bucket_seconds),
                    },
                )
            }
            AnalyzeCommands::Full => cheolsu_ops::analyze::analyze_full(&ctx),
        },

        // Replay (sequence만 — request/repeat는 위에서 처리)
        Commands::Replay(ReplayCommands::Sequence(args)) => {
            cheolsu_ops::replay::replay_sequence(
                &ctx,
                ReplaySequenceParams {
                    transaction_ids: args.ids,
                    delay_ms: Some(args.delay_ms),
                },
            )
            .await
        }
        Commands::Replay(_) => unreachable!(),

        // Session
        Commands::Session(cmd) => match cmd {
            SessionCommands::Save(args) => cheolsu_ops::session::save_session(
                &ctx,
                SaveSessionParams {
                    path: args.path,
                    filter: args.filter,
                    name: args.name,
                    description: args.description,
                },
            ),
            SessionCommands::Load(args) => cheolsu_ops::session::load_session(
                &ctx,
                LoadSessionParams {
                    path: args.path,
                    append: args.append,
                },
            ),
        },

        // Script
        Commands::Script(cmd) => match cmd {
            ScriptCommands::Load { path, code } => {
                cheolsu_ops::script::load_script(&ctx, LoadScriptParams { path, code }).await
            }
            ScriptCommands::Unload => cheolsu_ops::script::unload_script(&ctx).await,
        },

        // Export HAR
        Commands::ExportHar(args) => cheolsu_ops::export::export_har(
            &ctx,
            ExportHarParams {
                output_path: args.output_path,
                host: args.host,
                path: args.path,
            },
        ),

        Commands::Tui(_) => unreachable!(),
    };

    output::print_result(result)
}
