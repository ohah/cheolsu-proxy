fn main() {
    // macOS: OpenSSL dylib를 @rpath 기반으로 링크하도록 설정
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    }

    tauri_build::build()
}
