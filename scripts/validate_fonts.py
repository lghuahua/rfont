#!/usr/bin/env python3
"""用外部工具校验 rfont CLI 生成的字体。

流程：调用 CLI 对内置字体做子集化（ttf/woff/woff2），再把 woff2 转回 ttf，
然后用 fontTools 检查表结构、cmap 覆盖、OS/2 与 head 字段、校验和；
若系统安装了 ots-sanitize，则额外跑一遍 OTS 严格校验。

依赖：
    python -m pip install fonttools brotli

用法：
    python scripts/validate_fonts.py
    python scripts/validate_fonts.py --require-ots
"""

from __future__ import annotations

import argparse
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

from fontTools.ttLib import TTFont

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FONT = ROOT / "crates" / "rfont" / "src" / "AlimamaDaoLiTi.ttf"
DEFAULT_TEXT = "字体子集化测试ABCabc0123，。"

REQUIRED_TABLES = {
    "OS/2",
    "cmap",
    "glyf",
    "head",
    "hhea",
    "hmtx",
    "loca",
    "maxp",
    "name",
    "post",
}

SFNT_MAGIC = {b"\x00\x01\x00\x00", b"OTTO", b"true"}
MAGIC_BY_FORMAT = {
    "ttf": SFNT_MAGIC,
    "woff": {b"wOFF"},
    "woff2": {b"wOF2"},
}

# LONGDATETIME 自 1904 年起算；3_500_000_000 约等于 2015 年。
MIN_MODIFIED = 3_500_000_000


class Failure(Exception):
    pass


def calc_checksum(data: bytes) -> int:
    padded = data + b"\x00" * (-len(data) % 4)
    return sum(struct.unpack(f">{len(padded) // 4}I", padded)) & 0xFFFFFFFF


def verify_sfnt_checksums(data: bytes) -> None:
    """校验 sfnt 目录中各表校验和与整文件校验和。"""
    if data[:4] not in SFNT_MAGIC:
        raise Failure("不是 sfnt 字体")
    num_tables = struct.unpack(">H", data[4:6])[0]
    if len(data) < 12 + num_tables * 16:
        raise Failure("表目录被截断")

    head_entry = None
    for index in range(num_tables):
        base = 12 + index * 16
        tag = data[base : base + 4].decode("latin1")
        checksum, offset, length = struct.unpack(">III", data[base + 4 : base + 16])
        table = data[offset : offset + length]
        if len(table) != length:
            raise Failure(f"表 {tag!r} 超出文件末尾")
        if tag == "head":
            head_entry = (offset, length)
            table = table[:8] + b"\x00\x00\x00\x00" + table[12:]
        actual = calc_checksum(table)
        if actual != checksum:
            raise Failure(
                f"表 {tag!r} 校验和不匹配: 目录 {checksum:#010x}, 实际 {actual:#010x}"
            )

    if head_entry is None:
        raise Failure("缺少 head 表")
    total = calc_checksum(data)
    if total != 0xB1B0AFBA:
        raise Failure(f"整文件校验和应为 0xB1B0AFBA，实际 {total:#010x}")


def run_cli(cli: Path, args: list[str]) -> None:
    proc = subprocess.run(
        [str(cli), *args],
        capture_output=True,
        encoding="utf-8",
        errors="replace",
    )
    if proc.returncode != 0:
        raise Failure(
            "CLI 执行失败: "
            + " ".join([str(cli), *args])
            + f"\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )


def verify_font(
    path: Path,
    fmt: str,
    chars: list[str],
    source_glyph_count: int,
    check_checksums: bool,
) -> int:
    data = path.read_bytes()
    if data[:4] not in MAGIC_BY_FORMAT[fmt]:
        raise Failure(f"文件头不是 {fmt} 格式: {data[:4]!r}")

    if check_checksums:
        verify_sfnt_checksums(data)

    try:
        font = TTFont(str(path), checkChecksums=0)
    except ImportError as exc:
        raise Failure(f"无法解压 {fmt}: {exc}，请安装 brotli") from exc
    try:
        tables = {str(tag) for tag in font.keys()}
        missing = REQUIRED_TABLES - tables
        if missing:
            raise Failure(f"缺少必需表: {sorted(missing)}")

        expected_flavor = None if fmt == "ttf" else fmt
        if font.flavor != expected_flavor:
            raise Failure(f"flavor 应为 {expected_flavor!r}，实际 {font.flavor!r}")

        num_glyphs = font["maxp"].numGlyphs
        if num_glyphs == 0:
            raise Failure("maxp.numGlyphs 为 0")
        if num_glyphs >= source_glyph_count:
            raise Failure(
                f"子集字形数 {num_glyphs} 未少于原字体 {source_glyph_count}"
            )
        if len(font.getGlyphOrder()) != num_glyphs:
            raise Failure("字形顺序长度与 maxp.numGlyphs 不一致")

        cmap = font.getBestCmap() or {}
        missing_chars = [ch for ch in chars if ord(ch) not in cmap]
        if missing_chars:
            raise Failure(f"cmap 缺少字符: {''.join(missing_chars)!r}")

        codepoints = sorted(cmap)
        first = min(min(codepoints), 0xFFFF)
        last = min(max(codepoints), 0xFFFF)
        os2 = font["OS/2"]
        if (os2.usFirstCharIndex, os2.usLastCharIndex) != (first, last):
            raise Failure(
                "OS/2 字符范围不匹配: "
                f"({os2.usFirstCharIndex}, {os2.usLastCharIndex}) != ({first}, {last})"
            )

        head = font["head"]
        if head.modified < MIN_MODIFIED:
            raise Failure(f"head.modified 未更新: {head.modified}")

        if font["post"].formatType not in (2.0, 3.0):
            raise Failure(f"post 版本异常: {font['post'].formatType}")

        return num_glyphs
    finally:
        font.close()


def run_ots(path: Path) -> str:
    exe = shutil.which("ots-sanitize")
    if exe is None:
        return "skipped"
    proc = subprocess.run(
        [exe, str(path)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    if proc.returncode != 0:
        stderr = proc.stderr.decode("utf-8", errors="replace")
        raise Failure(f"OTS 校验失败:\n{stderr}")
    return "ok"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--cli",
        type=Path,
        default=ROOT / "target" / "release" / ("rfont.exe" if sys.platform == "win32" else "rfont"),
        help="rfont CLI 可执行文件路径",
    )
    parser.add_argument("--font", type=Path, default=DEFAULT_FONT, help="输入字体路径")
    parser.add_argument("--text", default=DEFAULT_TEXT, help="用于子集化的文本")
    parser.add_argument(
        "--require-ots",
        action="store_true",
        help="找不到 ots-sanitize 时视为失败（CI 使用）",
    )
    args = parser.parse_args()

    if not args.cli.is_file():
        print(f"[FAIL] 找不到 CLI: {args.cli}，请先 cargo build --release -p rfont-cli", file=sys.stderr)
        return 1
    if not args.font.is_file():
        print(f"[FAIL] 找不到字体: {args.font}", file=sys.stderr)
        return 1
    if shutil.which("ots-sanitize") is None:
        message = "[WARN] 未安装 ots-sanitize，跳过 OTS 校验"
        if args.require_ots:
            print(message, file=sys.stderr)
            return 1
        print(message)

    source = TTFont(str(args.font), checkChecksums=0)
    try:
        source_cmap = source.getBestCmap() or {}
        source_glyph_count = source["maxp"].numGlyphs
    finally:
        source.close()

    chars = sorted({ch for ch in args.text if ord(ch) in source_cmap})
    if len(chars) < 8:
        print(f"[FAIL] 文本中仅 {len(chars)} 个字符被原字体覆盖，无法校验", file=sys.stderr)
        return 1
    subset_text = "".join(chars)

    failures = 0
    with tempfile.TemporaryDirectory(prefix="rfont-validate-") as tmp:
        workdir = Path(tmp)
        outputs: dict[str, Path] = {}
        try:
            for fmt in ("ttf", "woff", "woff2"):
                output = workdir / f"subset.{fmt}"
                run_cli(
                    args.cli,
                    ["subset", str(args.font), "--text", subset_text, "-o", str(output), "--format", fmt],
                )
                outputs[fmt] = output

            roundtrip = workdir / "roundtrip.ttf"
            run_cli(
                args.cli,
                ["convert", str(outputs["woff2"]), "-o", str(roundtrip), "--format", "ttf"],
            )
            outputs["roundtrip"] = roundtrip

            for name in ("ttf", "woff", "woff2", "roundtrip"):
                path = outputs[name]
                fmt = "ttf" if name == "roundtrip" else name
                try:
                    glyphs = verify_font(
                        path,
                        fmt,
                        chars,
                        source_glyph_count,
                        check_checksums=(name in ("ttf", "roundtrip")),
                    )
                    ots = run_ots(path)
                    print(
                        f"[OK] {name:<9} glyphs={glyphs:<4} chars={len(chars):<3} "
                        f"checksums={'yes' if name in ('ttf', 'roundtrip') else 'n/a'} ots={ots}"
                    )
                except Failure as exc:
                    failures += 1
                    print(f"[FAIL] {name}: {exc}", file=sys.stderr)
        except Failure as exc:
            print(f"[FAIL] {exc}", file=sys.stderr)
            return 1

    if failures:
        print(f"\n外部校验失败: {failures} 个文件", file=sys.stderr)
        return 1
    print(f"\n外部校验通过: {len(outputs)} 个文件")
    return 0


if __name__ == "__main__":
    sys.exit(main())
