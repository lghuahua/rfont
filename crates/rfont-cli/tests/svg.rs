use std::path::PathBuf;
use std::process::Command;

fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rfont/src/AlimamaDaoLiTi.ttf")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rfont-cli-svg-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn svg_text_writes_document() {
    let dir = temp_dir("text");
    let output = dir.join("text.svg");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "svg",
            font_path().to_str().unwrap(),
            "--text",
            "AB",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let svg = std::fs::read_to_string(&output).unwrap();
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("<path d=\"M"));
}

#[test]
fn svg_glyph_respects_color_and_size() {
    let dir = temp_dir("glyph");
    let output = dir.join("glyph.svg");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "svg",
            font_path().to_str().unwrap(),
            "--glyph",
            "36",
            "--size",
            "128",
            "--color",
            "#ff0000",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    let svg = std::fs::read_to_string(&output).unwrap();
    assert!(svg.contains("fill=\"#ff0000\""));
    assert!(svg.contains("scale(0.128 -0.128)"));
}

#[test]
fn svg_requires_text_or_glyph() {
    let dir = temp_dir("empty");
    let output = dir.join("empty.svg");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "svg",
            font_path().to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(!status.success());
    assert!(!output.exists());
}

#[test]
fn svg_rejects_text_with_glyph() {
    let dir = temp_dir("both");
    let output = dir.join("both.svg");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "svg",
            font_path().to_str().unwrap(),
            "--text",
            "A",
            "--glyph",
            "36",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(!status.success());
    assert!(!output.exists());
}
