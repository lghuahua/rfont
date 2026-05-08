# PowerShell 脚本：自动生成 CHANGELOG
# 使用方法: .\scripts\generate-changelog.ps1

param(
    [switch]$Unreleased,  # 只生成未发布的变更
    [switch]$Preview,     # 预览而不保存
    [string]$Output = "CHANGELOG.md"  # 输出文件路径
)

Write-Host "🔧 正在生成 CHANGELOG..." -ForegroundColor Cyan

# 检查 git-cliff 是否安装
if (-not (Get-Command git-cliff -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 错误: git-cliff 未安装" -ForegroundColor Red
    Write-Host "请运行: cargo install git-cliff" -ForegroundColor Yellow
    exit 1
}

# 构建命令
$cmd = "git-cliff"
$args = @()

if ($Unreleased) {
    $args += "--unreleased"
    Write-Host "📝 生成未发布的变更..." -ForegroundColor Green
} else {
    Write-Host "📝 生成完整 CHANGELOG..." -ForegroundColor Green
}

if ($Preview) {
    # 预览模式：直接输出到控制台
    & $cmd @args
} else {
    # 保存到文件
    $args += "--output", $Output
    & $cmd @args
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ CHANGELOG 已生成: $Output" -ForegroundColor Green
        
        # 显示文件大小
        $file = Get-Item $Output
        $size = [math]::Round($file.Length / 1KB, 2)
        Write-Host "📊 文件大小: ${size} KB" -ForegroundColor Cyan
        
        # 显示行数
        $lines = (Get-Content $Output).Count
        Write-Host "📄 总行数: $lines" -ForegroundColor Cyan
    } else {
        Write-Host "❌ 生成失败" -ForegroundColor Red
        exit 1
    }
}

Write-Host "`n💡 提示:" -ForegroundColor Yellow
Write-Host "  - 使用 -Unreleased 参数只生成未发布的变更" -ForegroundColor Gray
Write-Host "  - 使用 -Preview 参数预览而不保存" -ForegroundColor Gray
Write-Host "  - 使用 -Output 指定输出文件路径" -ForegroundColor Gray
