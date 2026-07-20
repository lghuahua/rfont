#!/usr/bin/env node
/**
 * scripts/pre-release.js
 * cargo-release pre-release hook 脚本（Node.js 实现，跨平台）
 * 支持多 crate 场景，每个 crate 可以有独立的 CHANGELOG.md
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// 获取脚本所在目录的父目录（workspace 根目录）
const SCRIPT_DIR = __dirname;
const PROJECT_ROOT = path.join(SCRIPT_DIR, '..');

// 获取版本号（由 cargo-release 设置）
const NEW_VERSION = process.env.NEW_VERSION || 'unknown';
const DATE = new Date().toISOString().split('T')[0];

// 获取当前 crate 目录（cargo-release 执行时的目录）
const CURRENT_DIR = process.cwd();

// 判断是否在 crate 目录下执行
function getCrateRoot() {
    // 检查当前目录是否有 Cargo.toml 且不是 workspace 根目录
    const currentCargoToml = path.join(CURRENT_DIR, 'Cargo.toml');
    const workspaceCargoToml = path.join(PROJECT_ROOT, 'Cargo.toml');
    
    if (fs.existsSync(currentCargoToml) && CURRENT_DIR !== PROJECT_ROOT) {
        // 检查是否是 workspace 成员
        const cargoContent = fs.readFileSync(currentCargoToml, 'utf-8');
        if (!cargoContent.includes('[workspace]')) {
            // 特殊处理：如果当前目录是 src-tauri，向上查找父目录
            // 用于 rfont-desktop 项目（路径：rfont-desktop/src-tauri）
            const relativePath = path.relative(PROJECT_ROOT, CURRENT_DIR);
            if (relativePath.endsWith('src-tauri')) {
                const parentDir = path.dirname(CURRENT_DIR);
                // 验证父目录是否有 Cargo.toml（虽然不是 workspace）
                // 或者父目录是项目根目录的合理位置
                if (fs.existsSync(parentDir)) {
                    return parentDir;  // 返回父目录（如 rfont-desktop/）
                }
            }
            return CURRENT_DIR;
        }
    }
    return null;
}

// 确定 CHANGELOG 路径
const CRATE_ROOT = getCrateRoot();
const CHANGELOG_PATH = CRATE_ROOT
    ? path.join(CRATE_ROOT, 'CHANGELOG.md')
    : path.join(PROJECT_ROOT, 'CHANGELOG.md');

const DISPLAY_ROOT = CRATE_ROOT || PROJECT_ROOT;

console.log('==========================================');
console.log('  Pre-release hook starting');
console.log(`  Version: v${NEW_VERSION}`);
console.log(`  Target: ${CRATE_ROOT ? path.relative(PROJECT_ROOT, CRATE_ROOT) : 'workspace root'}`);
console.log(`  Project root: ${DISPLAY_ROOT}`);
console.log('==========================================');

// 防止重复执行：检查 CHANGELOG 中是否已包含当前版本
if (fs.existsSync(CHANGELOG_PATH)) {
    const changelogContent = fs.readFileSync(CHANGELOG_PATH, 'utf-8');
    if (changelogContent.includes(`## [${NEW_VERSION}]`)) {
        console.log(`⚠️  Version ${NEW_VERSION} already in CHANGELOG, skipping...`);
        process.exit(0);
    }
}

// 步骤 1: 使用 git-cliff 生成 CHANGELOG
console.log('');
console.log('📝 Generating CHANGELOG from git history...');

// 检查 git-cliff 是否安装
try {
    execSync('git-cliff --version', { stdio: 'ignore' });
} catch (error) {
    console.log('❌ git-cliff not found. Installing...');
    try {
        execSync('cargo install git-cliff', { stdio: 'inherit' });
    } catch (installError) {
        console.error('❌ Failed to install git-cliff');
        process.exit(1);
    }
}

// 生成未发布的内容到临时文件（不包含 header 和 footer）
const UNRELEASED_PATH = path.join(DISPLAY_ROOT, 'CHANGELOG_UNRELEASED.md');
try {
    execSync(`git-cliff --unreleased --strip all --output "${UNRELEASED_PATH}"`, {
        cwd: DISPLAY_ROOT,
        stdio: 'inherit'
    });
    console.log('✅ CHANGELOG content generated');
} catch (error) {
    console.error('❌ Failed to generate CHANGELOG');
    process.exit(1);
}

// 步骤 2: 更新 CHANGELOG.md
console.log('');
console.log('🔖 Updating CHANGELOG.md...');

if (!fs.existsSync(UNRELEASED_PATH)) {
    console.error('❌ Error: CHANGELOG_UNRELEASED.md not found');
    process.exit(1);
}

// 如果 CHANGELOG.md 不存在，创建初始版本
if (!fs.existsSync(CHANGELOG_PATH)) {
    console.log('📝 CHANGELOG.md not found, creating new one...');
    const initialChangelog = `# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

`;
    fs.writeFileSync(CHANGELOG_PATH, initialChangelog, 'utf-8');
    console.log('✅ CHANGELOG.md created');
}

// 读取临时文件内容
let newContent = fs.readFileSync(UNRELEASED_PATH, 'utf-8');

// 移除第一行（## [Unreleased]）和开头的空行
const lines = newContent.split('\n');
// 移除第一行如果它是 ## [Unreleased]
if (lines[0] && lines[0].trim().startsWith('## [Unreleased]')) {
    lines.shift();
}
// 移除开头的空行
while (lines.length > 0 && lines[0].trim() === '') {
    lines.shift();
}
newContent = lines.join('\n');

// 创建版本标题
const VERSION_HEADER = `## [${NEW_VERSION}] - ${DATE}`;

// 读取 CHANGELOG.md 内容
let changelogContent = fs.readFileSync(CHANGELOG_PATH, 'utf-8');

// 在 [Unreleased] 后面插入新版本
if (changelogContent.includes('## [Unreleased]')) {
    // 使用正则表达式在 ## [Unreleased] 后插入内容
    const regex = /(## \[Unreleased\])/;
    const match = changelogContent.match(regex);
    
    if (match) {
        const insertIndex = match.index + match[0].length;
        const newChangelog = 
            changelogContent.slice(0, insertIndex) +
            '\n\n' +
            VERSION_HEADER +
            '\n\n' +
            newContent +
            '\n' +
            changelogContent.slice(insertIndex);
        
        fs.writeFileSync(CHANGELOG_PATH, newChangelog, 'utf-8');
        console.log('✅ CHANGELOG.md updated');
    } else {
        console.log('⚠️  Warning: [Unreleased] marker not found');
    }
} else {
    console.log('⚠️  Warning: [Unreleased] marker not found');
}

// 清理临时文件
try {
    fs.unlinkSync(UNRELEASED_PATH);
} catch (error) {
    // 忽略删除错误
}

console.log('');
console.log('==========================================');
console.log('  Pre-release hook completed successfully!');
console.log('==========================================');
