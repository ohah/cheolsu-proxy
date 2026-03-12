use crate::handler::LoggingHandler;

#[test]
fn test_guess_content_type_json() {
    assert_eq!(
        LoggingHandler::guess_content_type("data.json"),
        "application/json"
    );
}

#[test]
fn test_guess_content_type_html() {
    assert_eq!(
        LoggingHandler::guess_content_type("index.html"),
        "text/html; charset=utf-8"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("page.htm"),
        "text/html; charset=utf-8"
    );
}

#[test]
fn test_guess_content_type_javascript() {
    assert_eq!(
        LoggingHandler::guess_content_type("app.js"),
        "application/javascript"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("module.mjs"),
        "application/javascript"
    );
}

#[test]
fn test_guess_content_type_css() {
    assert_eq!(LoggingHandler::guess_content_type("style.css"), "text/css");
}

#[test]
fn test_guess_content_type_images() {
    assert_eq!(LoggingHandler::guess_content_type("photo.png"), "image/png");
    assert_eq!(
        LoggingHandler::guess_content_type("photo.jpg"),
        "image/jpeg"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("photo.jpeg"),
        "image/jpeg"
    );
    assert_eq!(LoggingHandler::guess_content_type("anim.gif"), "image/gif");
    assert_eq!(
        LoggingHandler::guess_content_type("icon.svg"),
        "image/svg+xml"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("photo.webp"),
        "image/webp"
    );
}

#[test]
fn test_guess_content_type_fonts() {
    assert_eq!(
        LoggingHandler::guess_content_type("font.woff"),
        "font/woff2"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("font.woff2"),
        "font/woff2"
    );
}

#[test]
fn test_guess_content_type_other() {
    assert_eq!(
        LoggingHandler::guess_content_type("file.bin"),
        "application/octet-stream"
    );
    assert_eq!(
        LoggingHandler::guess_content_type("noext"),
        "application/octet-stream"
    );
}

#[test]
fn test_guess_content_type_xml() {
    assert_eq!(
        LoggingHandler::guess_content_type("data.xml"),
        "application/xml"
    );
}

#[test]
fn test_guess_content_type_txt() {
    assert_eq!(
        LoggingHandler::guess_content_type("readme.txt"),
        "text/plain; charset=utf-8"
    );
}

#[test]
fn test_guess_content_type_pdf() {
    assert_eq!(
        LoggingHandler::guess_content_type("doc.pdf"),
        "application/pdf"
    );
}

#[test]
fn test_guess_content_type_case_insensitive() {
    assert_eq!(
        LoggingHandler::guess_content_type("DATA.JSON"),
        "application/json"
    );
    assert_eq!(LoggingHandler::guess_content_type("STYLE.CSS"), "text/css");
}

#[test]
fn test_guess_content_type_with_path() {
    assert_eq!(
        LoggingHandler::guess_content_type("/var/data/response.json"),
        "application/json"
    );
}
