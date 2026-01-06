use redis_nav::format::{detect_format, DetectedFormat};

#[test]
fn test_detect_json_object() {
    let json = r#"{"name": "test", "value": 123}"#;
    assert_eq!(detect_format(json.as_bytes()), DetectedFormat::Json);
}

#[test]
fn test_detect_json_array() {
    let json = r#"[1, 2, 3]"#;
    assert_eq!(detect_format(json.as_bytes()), DetectedFormat::Json);
}

#[test]
fn test_detect_xml() {
    let xml = r#"<?xml version="1.0"?><root></root>"#;
    assert_eq!(detect_format(xml.as_bytes()), DetectedFormat::Xml);
}

#[test]
fn test_detect_html() {
    let html = r#"<html><body></body></html>"#;
    assert_eq!(detect_format(html.as_bytes()), DetectedFormat::Html);
}

#[test]
fn test_detect_plain_text() {
    let text = "Hello, world!";
    assert_eq!(detect_format(text.as_bytes()), DetectedFormat::PlainText);
}

#[test]
fn test_detect_binary_png() {
    let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    assert_eq!(detect_format(&png_header), DetectedFormat::Binary);
}
