# Cheolsu Proxy CLI Command

Execute Cheolsu Proxy CLI commands directly from the conversation.

## Usage

`/cheolsu <subcommand> [options]`

If no arguments are provided, check proxy status.

## Execution

1. Run the user-provided arguments as a `cheolsu` CLI command.
2. If no arguments, run `cheolsu status`.
3. Show the result to the user.

### Command

```bash
cheolsu $ARGUMENTS
```

If arguments are empty:
```bash
cheolsu status
```

## Examples

- `/cheolsu` — 프록시 상태 확인 / Check proxy status
- `/cheolsu traffic search --host example.com` — 트래픽 검색 / Search traffic
- `/cheolsu rule list` — 규칙 목록 / List intercept rules
- `/cheolsu rule add --name "Block ads" --pattern "*.ads.com" --action-type block` — 규칙 추가 / Add rule
- `/cheolsu analyze full` — 전체 분석 / Full traffic analysis
- `/cheolsu replay request --method GET --url https://httpbin.org/get` — HTTP 요청 / Send HTTP request
- `/cheolsu session save ./traffic.cheolsu` — 세션 저장 / Save session
- `/cheolsu export-har ./output.har` — HAR 내보내기 / Export HAR

## Available Subcommands

| Subcommand | Description |
|-----------|-------------|
| `traffic search/get/ws/sse/diff/clear/openapi` | Search, inspect, compare captured traffic |
| `rule list/add/remove` | Manage intercept rules (block, modify, map) |
| `breakpoint list/add/remove/pending/resolve` | Manage breakpoints |
| `host-mapping list/add/remove` | Manage DNS spoofing / host mapping |
| `reverse-proxy list/add/remove` | Manage reverse proxy rules |
| `server-replay list/add/remove/clear` | Manage server replay entries |
| `settings upstream-proxy/throttle/proxy-auth/connection-strategy/quick` | Configure proxy settings |
| `analyze performance/errors/endpoints/duplicates/n-plus1/timeline/full` | Analyze traffic |
| `replay request/sequence/repeat` | Send HTTP requests |
| `session save/load` | Save/load traffic sessions |
| `script load/unload` | Manage proxy scripts |
| `status` | Check proxy daemon status |
| `export-har` | Export traffic as HAR file |
| `tui` | Launch TUI mode |

## Error Handling

- If the daemon is not running, an error message will be displayed.
  - Guide the user to start the proxy first.
- If `cheolsu` binary is not in PATH, suggest `cargo build -p cheolsu-cli`.
