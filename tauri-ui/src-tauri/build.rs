use std::process::Command;

fn main() {
    // MCP 서버 바이너리 빌드 및 복사
    build_mcp_server();

    tauri_build::build()
}

fn build_mcp_server() {
    let target_triple = std::env::var("TARGET").expect("TARGET env not set");
    let profile = std::env::var("PROFILE").expect("PROFILE env not set");

    let cargo_args = if profile == "release" {
        vec!["build", "-p", "cheolsu-proxy-mcp", "--release"]
    } else {
        vec!["build", "-p", "cheolsu-proxy-mcp"]
    };

    // workspace root는 src-tauri의 두 단계 위
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let status = Command::new("cargo")
        .args(&cargo_args)
        .current_dir(workspace_root)
        .status()
        .expect("Failed to run cargo build for cheolsu-proxy-mcp");

    if !status.success() {
        panic!("Failed to build cheolsu-proxy-mcp");
    }

    let profile_dir = if profile == "release" {
        "release"
    } else {
        "debug"
    };

    let src = workspace_root
        .join("target")
        .join(profile_dir)
        .join("cheolsu-proxy-mcp");

    let dest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries");
    std::fs::create_dir_all(&dest_dir).expect("Failed to create binaries directory");

    let dest = dest_dir.join(format!("cheolsu-proxy-mcp-{target_triple}"));
    std::fs::copy(&src, &dest).unwrap_or_else(|e| {
        panic!(
            "Failed to copy MCP binary from {} to {}: {}",
            src.display(),
            dest.display(),
            e
        )
    });

    println!("cargo::rerun-if-changed=../../crates/mcp_server/src/");
    println!("cargo::rerun-if-changed=../../crates/mcp_server/Cargo.toml");
}
