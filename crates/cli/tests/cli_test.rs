use assert_cmd::Command;
use predicates::prelude::*;

fn cheolsu() -> Command {
    Command::cargo_bin("cheolsu").unwrap()
}

// ─── Help output ─────────────────────────────────────────

#[test]
fn help_shows_all_subcommands() {
    cheolsu()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("traffic"))
        .stdout(predicate::str::contains("rule"))
        .stdout(predicate::str::contains("breakpoint"))
        .stdout(predicate::str::contains("host-mapping"))
        .stdout(predicate::str::contains("reverse-proxy"))
        .stdout(predicate::str::contains("server-replay"))
        .stdout(predicate::str::contains("settings"))
        .stdout(predicate::str::contains("analyze"))
        .stdout(predicate::str::contains("replay"))
        .stdout(predicate::str::contains("session"))
        .stdout(predicate::str::contains("script"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("export-har"))
        .stdout(predicate::str::contains("tui"))
        .stdout(predicate::str::contains("install-skills"))
        .stdout(predicate::str::contains("uninstall-skills"));
}

#[test]
fn traffic_help() {
    cheolsu()
        .args(["traffic", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("ws"))
        .stdout(predicate::str::contains("sse"))
        .stdout(predicate::str::contains("diff"))
        .stdout(predicate::str::contains("clear"))
        .stdout(predicate::str::contains("openapi"));
}

#[test]
fn rule_help() {
    cheolsu()
        .args(["rule", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn analyze_help() {
    cheolsu()
        .args(["analyze", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("performance"))
        .stdout(predicate::str::contains("errors"))
        .stdout(predicate::str::contains("endpoints"))
        .stdout(predicate::str::contains("duplicates"))
        .stdout(predicate::str::contains("n-plus1"))
        .stdout(predicate::str::contains("timeline"))
        .stdout(predicate::str::contains("full"));
}

#[test]
fn settings_help() {
    cheolsu()
        .args(["settings", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("upstream-proxy"))
        .stdout(predicate::str::contains("throttle"))
        .stdout(predicate::str::contains("proxy-auth"))
        .stdout(predicate::str::contains("connection-strategy"))
        .stdout(predicate::str::contains("quick"));
}

#[test]
fn replay_help() {
    cheolsu()
        .args(["replay", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("request"))
        .stdout(predicate::str::contains("sequence"))
        .stdout(predicate::str::contains("repeat"));
}

#[test]
fn tui_help() {
    cheolsu()
        .args(["tui", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--port"))
        .stdout(predicate::str::contains("--host"));
}

// ─── No daemon error ─────────────────────────────────────

#[test]
fn status_without_daemon_shows_error() {
    cheolsu()
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("데몬"));
}

#[test]
fn rule_list_without_daemon_shows_error() {
    cheolsu()
        .args(["rule", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("데몬"));
}

#[test]
fn traffic_search_without_daemon_shows_error() {
    cheolsu()
        .args(["traffic", "search"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("데몬"));
}

// ─── Invalid arguments ───────────────────────────────────

#[test]
fn unknown_subcommand_shows_error() {
    cheolsu()
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn rule_add_missing_required_args() {
    cheolsu()
        .args(["rule", "add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn replay_request_missing_required_args() {
    cheolsu()
        .args(["replay", "request"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// ─── Skills install/uninstall ────────────────────────────

#[test]
fn install_skills_succeeds() {
    cheolsu()
        .arg("install-skills")
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"))
        .stdout(predicate::str::contains("cheolsu.md"))
        .stdout(predicate::str::contains("dev-url-mapping.md"));
}

#[test]
fn uninstall_skills_succeeds() {
    // install first, then uninstall
    cheolsu().arg("install-skills").assert().success();
    cheolsu()
        .arg("uninstall-skills")
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"))
        .stdout(predicate::str::contains("uninstalled"));
}
