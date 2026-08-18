# build-release.ps1 - 构建 dsh-desktop 免安装包
# 用法: .\scripts\build-release.ps1

param(
    [string]$Version = "0.0.1",
    [string]$SourceDir = "D:\AI\project\dsh-portable"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$ReleaseDir = "$Root\release\dsh-desktop"

Write-Host "=== dsh-desktop v$Version 构建 ===" -ForegroundColor Cyan
Write-Host "源目录: $SourceDir"
Write-Host "输出目录: $ReleaseDir"

# 1. 编译 exe
Write-Host "`n[1/6] 编译 exe..." -ForegroundColor Yellow
Push-Location "$SourceDir\src-tauri"
$env:Path = "C:\Users\Administrator\.cargo\bin;$env:Path"
cargo build --release 2>&1 | Select-Object -Last 5
Pop-Location

# 2. 创建 release 目录
Write-Host "`n[2/6] 创建 release 目录..." -ForegroundColor Yellow
if (Test-Path $ReleaseDir) { Remove-Item $ReleaseDir -Recurse -Force }
New-Item -Path $ReleaseDir -ItemType Directory -Force | Out-Null
New-Item -Path "$ReleaseDir\ui" -ItemType Directory -Force | Out-Null
New-Item -Path "$ReleaseDir\runtime\node" -ItemType Directory -Force | Out-Null
New-Item -Path "$ReleaseDir\runtime\dsh" -ItemType Directory -Force | Out-Null
New-Item -Path "$ReleaseDir\data" -ItemType Directory -Force | Out-Null

# 3. 复制 exe
Write-Host "`n[3/6] 复制 exe..." -ForegroundColor Yellow
$ExeSrc = "$SourceDir\src-tauri\target\release\dsh-portable.exe"
if (-not (Test-Path $ExeSrc)) {
    Write-Host "exe 不存在，尝试 debug 版本..." -ForegroundColor Red
    $ExeSrc = "$SourceDir\src-tauri\target\debug\dsh-portable.exe"
}
Copy-Item $ExeSrc "$ReleaseDir\dsh-desktop.exe"
Write-Host "  复制: dsh-desktop.exe ($([math]::Round((Get-Item $ExeSrc).Length/1MB, 1)) MB)"

# 4. 复制 ui/index.html
Write-Host "`n[4/6] 复制 ui..." -ForegroundColor Yellow
Copy-Item "$SourceDir\ui\index.html" "$ReleaseDir\ui\index.html"
Write-Host "  复制: ui/index.html"

# 5. 复制 runtime
Write-Host "`n[5/6] 复制 runtime..." -ForegroundColor Yellow
Copy-Item "$SourceDir\runtime\node\node.exe" "$ReleaseDir\runtime\node\node.exe"
Write-Host "  复制: runtime/node/node.exe ($([math]::Round((Get-Item "$SourceDir\runtime\node\node.exe").Length/1MB, 1)) MB)"

Write-Host "  复制: runtime/dsh/ (node_modules)..." -NoNewline
Copy-Item "$SourceDir\runtime\dsh\*" "$ReleaseDir\runtime\dsh\" -Recurse -Force
$dshSize = (Get-ChildItem "$ReleaseDir\runtime\dsh" -Recurse -File | Measure-Object -Property Length -Sum).Sum
Write-Host " ($([math]::Round($dshSize/1MB, 1)) MB)"

# 6. 创建空 data 结构
Write-Host "`n[6/6] 创建 data 目录结构..." -ForegroundColor Yellow
@("profiles", "sessions", "storages", "logs") | ForEach-Object {
    New-Item -Path "$ReleaseDir\data\$_" -ItemType Directory -Force | Out-Null
}

# 统计
$TotalSize = (Get-ChildItem $ReleaseDir -Recurse -File | Measure-Object -Property Length -Sum).Sum
$FileCount = (Get-ChildItem $ReleaseDir -Recurse -File).Count

Write-Host "`n=== 构建完成 ===" -ForegroundColor Green
Write-Host "输出: $ReleaseDir"
Write-Host "大小: $([math]::Round($TotalSize/1MB, 1)) MB ($FileCount files)"

# 打包 zip
$ZipPath = "$Root\release\dsh-desktop-v$Version.zip"
Write-Host "`n打包: $ZipPath" -ForegroundColor Yellow
Compress-Archive -Path $ReleaseDir -DestinationPath $ZipPath -Force
$ZipSize = (Get-Item $ZipPath).Length
Write-Host "完成: $([math]::Round($ZipSize/1MB, 1)) MB" -ForegroundColor Green
