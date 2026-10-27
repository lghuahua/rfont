use std::path::PathBuf;
use std::process::Command;

fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rfont/src/AlimamaDaoLiTi.ttf")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rfont-cli-edit-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn edit_writes_renamed_font() {
    let dir = temp_dir("renamed");
    let output = dir.join("renamed.ttf");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "edit",
            font_path().to_str().unwrap(),
            "--family",
            "Renamed Font",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert!(output.exists());

    let info = rfont::Font::load(&output).unwrap().get_font_info().unwrap();
    assert_eq!(info.family.as_deref(), Some("Renamed Font"));
    assert_eq!(info.full_name.as_deref(), Some("Renamed Font Regular"));
    assert_eq!(info.postscript_name.as_deref(), Some("RenamedFont-Regular"));
}

#[test]
fn edit_preserves_glyph_count() {
    let dir = temp_dir("designer");
    let output = dir.join("designer.ttf");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "edit",
            font_path().to_str().unwrap(),
            "--designer",
            "Alice",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());

    let original = rfont::Font::load(font_path()).unwrap();
    let edited = rfont::Font::load(&output).unwrap();

    assert_eq!(
        original.get_font_info().unwrap().glyph_count,
        edited.get_font_info().unwrap().glyph_count
    );
    assert_eq!(
        edited.get_font_info().unwrap().designer.as_deref(),
        Some("Alice")
    );
}

#[test]
fn edit_rejects_invalid_postscript_name() {
    let dir = temp_dir("invalid");
    let output = dir.join("invalid.ttf");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "edit",
            font_path().to_str().unwrap(),
            "--postscript-name",
            "Bad Name",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(!status.success());
    assert!(!output.exists());
}

#[test]
fn edit_requires_at_least_one_field() {
    let dir = temp_dir("empty");
    let output = dir.join("empty.ttf");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "edit",
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
fn edit_outputs_woff2() {
    let dir = temp_dir("woff2");
    let output = dir.join("renamed.woff2");

    let status = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "edit",
            font_path().to_str().unwrap(),
            "--family",
            "Woff Family",
            "-f",
            "woff2",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(&std::fs::read(&output).unwrap()[0..4], b"wOF2");

    let info = rfont::Font::load(&output).unwrap().get_font_info().unwrap();
    assert_eq!(info.family.as_deref(), Some("Woff Family"));
}
