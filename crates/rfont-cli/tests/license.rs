use std::path::{Path, PathBuf};
use std::process::Command;

fn font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rfont/src/AlimamaDaoLiTi.ttf")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("rfont-cli-license-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 复制字体并把 OS/2.fsType 改成指定值。
///
/// 目录校验和不会重算，但 rfont 读取时不校验校验和。
fn restricted_font(dir: &Path, name: &str, fs_type: u16) -> PathBuf {
    let data = std::fs::read(font_path()).unwrap();
    let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;
    let mut patched = data;

    for index in 0..num_tables {
        let record = 12 + index * 16;
        if &patched[record..record + 4] == b"OS/2" {
            let offset = u32::from_be_bytes([
                patched[record + 8],
                patched[record + 9],
                patched[record + 10],
                patched[record + 11],
            ]) as usize;
            patched[offset + 8..offset + 10].copy_from_slice(&fs_type.to_be_bytes());

            let path = dir.join(name);
            std::fs::write(&path, &patched).unwrap();
            return path;
        }
    }

    panic!("OS/2 表不存在");
}

#[test]
fn subset_warns_but_succeeds_on_restricted_font() {
    let dir = temp_dir("warn");
    let input = restricted_font(&dir, "no-subset.ttf", 0x0100);
    let output = dir.join("out.ttf");

    let result = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "subset",
            input.to_str().unwrap(),
            "--text",
            "Hello",
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(result.status.success());
    assert!(output.exists());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("禁止子集化"), "stderr: {stderr}");
}

#[test]
fn subset_strict_aborts_on_restricted_font() {
    let dir = temp_dir("strict");
    let input = restricted_font(&dir, "no-subset.ttf", 0x0100);
    let output = dir.join("out.ttf");

    let result = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "subset",
            input.to_str().unwrap(),
            "--text",
            "Hello",
            "--strict",
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(!output.exists());
}

#[test]
fn subset_without_restrictions_has_no_warning() {
    let dir = temp_dir("clean");
    let output = dir.join("out.ttf");

    let result = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "subset",
            font_path().to_str().unwrap(),
            "--text",
            "Hello",
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(result.status.success());
    assert!(result.stderr.is_empty(), "stderr: {result:?}");
}

#[test]
fn edit_keeps_embedding_permission() {
    let dir = temp_dir("edit");
    let input = restricted_font(&dir, "restricted.ttf", 0x0002);
    let output = dir.join("edited.ttf");

    let result = Command::new(env!("CARGO_BIN_EXE_rfont"))
        .args([
            "edit",
            input.to_str().unwrap(),
            "--family",
            "Renamed",
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("禁止嵌入"), "stderr: {stderr}");

    let info = rfont::Font::load(&output).unwrap().get_font_info().unwrap();
    assert_eq!(info.fs_type, 0x0002);
    assert_eq!(info.family.as_deref(), Some("Renamed"));
}
