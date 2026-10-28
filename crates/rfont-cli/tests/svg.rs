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

    let render = |name: &str, size: &str| -> String {
        let output = dir.join(name);
        let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
            .args([
                "svg",
                font_path().to_str().unwrap(),
                "--glyph",
                "36",
                "--size",
                size,
                "--color",
                "#ff0000",
                "--padding",
                "0",
                "-o",
                output.to_str().unwrap(),
            ])
            .status()
            .unwrap();

        assert!(status.success());
        std::fs::read_to_string(&output).unwrap()
    };

    let small = render("small.svg", "64");
    let large = render("large.svg", "128");
    assert!(large.contains("fill=\"#ff0000\""));

    // 留白为 0 时画布尺寸与字号成正比，同时能确认 path 使用了相同的比例
    let width = |svg: &str| -> f64 {
        let start = svg.find("width=\"").unwrap() + 7;
        let end = svg[start..].find('"').unwrap() + start;
        svg[start..end].parse().unwrap()
    };
    assert!((width(&large) - width(&small) * 2.0).abs() < 0.01);
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
