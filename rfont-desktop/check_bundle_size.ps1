# 检查桌面应用体积.ps1
# 用于验证优化效果

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "检查 Tauri 应用体积" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$bundlePath = "src-tauri\target\release\bundle"

if (-not (Test-Path $bundlePath)) {
    Write-Host "❌ 未找到构建产物" -ForegroundColor Red
    Write-Host ""
    Write-Host "请先运行: pnpm tauri build" -ForegroundColor Yellow
    exit 1
}

Write-Host "📁 构建产物位置: $bundlePath" -ForegroundColor Green
Write-Host ""

# 查找所有 exe 文件
$exeFiles = Get-ChildItem -Path $bundlePath -Filter "*.exe" -Recurse -ErrorAction SilentlyContinue

if ($exeFiles) {
    Write-Host "📦 EXE 文件大小:" -ForegroundColor Cyan
    Write-Host ""
    
    $exeFiles | ForEach-Object {
        $sizeMB = [math]::Round($_.Length / 1MB, 2)
        $relativePath = $_.FullName.Replace($PWD.Path, "").TrimStart("\")
        
        if ($sizeMB -lt 3) {
            $color = "Green"
            $status = "✅ 优秀"
        } elseif ($sizeMB -lt 5) {
            $color = "Yellow"
            $status = "⚠️  良好"
        } else {
            $color = "Red"
            $status = "❌ 偏大"
        }
        
        Write-Host "  $relativePath" -ForegroundColor White
        Write-Host "    大小: $sizeMB MB - $status" -ForegroundColor $color
        Write-Host ""
    }
} else {
    Write-Host "❌ 未找到 EXE 文件" -ForegroundColor Red
}

# 计算总大小
Write-Host "📊 Bundle 目录总大小:" -ForegroundColor Cyan
$totalSize = (Get-ChildItem -Path $bundlePath -Recurse -File -ErrorAction SilentlyContinue | 
              Measure-Object -Property Length -Sum).Sum
$totalSizeMB = [math]::Round($totalSize / 1MB, 2)

if ($totalSizeMB -lt 5) {
    $color = "Green"
    $status = "✅ 优秀"
} elseif ($totalSizeMB -lt 10) {
    $color = "Yellow"
    $status = "⚠️  良好"
} else {
    $color = "Red"
    $status = "❌ 偏大"
}

Write-Host "  总计: $totalSizeMB MB - $status" -ForegroundColor $color
Write-Host ""

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "优化建议:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

if ($totalSizeMB -gt 5) {
    Write-Host "⚠️  体积仍然较大，可以尝试:" -ForegroundColor Yellow
    Write-Host "  1. 确认使用了 pnpm tauri build" -ForegroundColor White
    Write-Host "  2. 检查 Cargo.toml 中的 profile.release 配置" -ForegroundColor White
    Write-Host "  3. 运行 cargo clean 后重新构建" -ForegroundColor White
    Write-Host "  4. 查看 BUNDLE_SIZE_OPTIMIZATION.md 获取更多优化建议" -ForegroundColor White
} else {
    Write-Host "✅ 体积优化成功！" -ForegroundColor Green
}

Write-Host ""
