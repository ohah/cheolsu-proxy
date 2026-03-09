fn main() {
    // macOS: 테스트/바이너리가 openssl-bundle의 @rpath dylib을 찾을 수 있도록 rpath 추가
    // Tauri 앱은 자체 build.rs에서 @executable_path/../Frameworks rpath를 추가하므로
    // 런타임에 Frameworks/ 폴더의 dylib을 우선 사용함 (이 rpath는 fallback)
    #[cfg(target_os = "macos")]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let openssl_lib = format!(
            "{}/../../desktop/src-tauri/openssl-bundle/lib",
            manifest_dir
        );
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", openssl_lib);
    }
}
