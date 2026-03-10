// 트레이 ↔ 메인 윈도우 동기화는 Tauri 이벤트(emit/listen)로 처리합니다.
// tauri-plugin-store의 내부 std::sync::Mutex가 WebKit URL scheme handler와
// 메인 스레드에서 데드락을 유발할 수 있어 store 기반 동기화를 제거했습니다.
