# 快速生成覆盖率报告脚本（使用 cargo-llvm-cov）
# 
# 用法:
#   .\scripts\coverage.ps1              # 生成完整覆盖率报告
#   .\scripts\coverage.ps1 -Html        # 生成 HTML 报告
#   .\scripts\coverage.ps1 -Open        # 生成并打开 HTML 报告
#   .\scripts\coverage.ps1 -Json        # 生成 JSON 报告

param(
    [switch]$Html,
    [switch]$Open,
    [switch]$Json
)

$ErrorActionPreference = "Stop"

# 检查 cargo-llvm-cov 是否安装
if (-not (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 错误: cargo-llvm-cov 未安装" -ForegroundColor Red
    Write-Host "💡 请运行: cargo install cargo-llvm-cov --locked" -ForegroundColor Yellow
    exit 1
}

Write-Host "🚀 开始生成覆盖率报告..." -ForegroundColor Cyan
Write-Host ""

# 生成 LCOV 格式（用于 Codecov）
Write-Host "📊 生成 LCOV 报告..." -ForegroundColor Cyan
& cargo llvm-cov `
    --workspace `
    --lcov `
    --output-path lcov.info `
    --ignore-filename-regex "tests|benches|examples" `
    --no-fail-fast `
    --quiet

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ LCOV 报告已生成: lcov.info" -ForegroundColor Green
} else {
    Write-Host "❌ LCOV 报告生成失败" -ForegroundColor Red
    exit 1
}
Write-Host ""

# 生成 HTML 报告
if ($Html -or $Open) {
    Write-Host "📊 生成 HTML 报告..." -ForegroundColor Cyan
    & cargo llvm-cov `
        --workspace `
        --html `
        --output-dir coverage/html `
        --ignore-filename-regex "tests|benches|examples" `
        --no-fail-fast `
        --quiet
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ HTML 报告已生成: coverage/html/index.html" -ForegroundColor Green
        
        if ($Open) {
            Write-Host "🌐 正在打开 HTML 报告..." -ForegroundColor Cyan
            Start-Process "coverage/html/index.html"
        }
    } else {
        Write-Host "❌ HTML 报告生成失败" -ForegroundColor Red
    }
    Write-Host ""
}

# 生成 JSON 报告
if ($Json) {
    Write-Host "📊 生成 JSON 报告..." -ForegroundColor Cyan
    & cargo llvm-cov `
        --workspace `
        --json `
        --output-path coverage/report.json `
        --ignore-filename-regex "tests|benches|examples" `
        --no-fail-fast `
        --quiet
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ JSON 报告已生成: coverage/report.json" -ForegroundColor Green
    } else {
        Write-Host "❌ JSON 报告生成失败" -ForegroundColor Red
    }
    Write-Host ""
}

# 显示覆盖率摘要
Write-Host "📈 覆盖率摘要:" -ForegroundColor Cyan
& cargo llvm-cov `
    --workspace `
    --summary-only `
    --ignore-filename-regex "tests|benches|examples" `
    --no-fail-fast

Write-Host ""
Write-Host "✨ 完成！" -ForegroundColor Green
