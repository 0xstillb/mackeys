<#
.SYNOPSIS
    สร้างไฟล์ installer (setup.exe) ของ MacKeys ด้วย Inno Setup

.DESCRIPTION
    ต้องมี Inno Setup 6 ติดตั้งไว้ — ดาวน์โหลดได้ที่ https://jrsoftware.org/isdl.php
    (ถ้าติดตั้งแบบ per-user จะอยู่ที่ %LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe)

    ขั้นตอน: build release -> สร้างไอคอน .ico -> คอมไพล์สคริปต์ installer/mackeys.iss
    ผลลัพธ์อยู่ในโฟลเดอร์ dist\

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1 -Iscc "C:\Program Files\Inno Setup 6\ISCC.exe"
#>
param(
    # ระบุ path ของ ISCC.exe เอง (ถ้าเว้นว่างจะหาอัตโนมัติ)
    [string]$Iscc = '',
    # ข้ามขั้นตอน cargo build (ใช้เมื่อ build ไว้แล้ว)
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Find-Iscc {
    param([string]$Explicit)

    if ($Explicit) {
        if (Test-Path -LiteralPath $Explicit) { return $Explicit }
        throw "ไม่พบ ISCC.exe ที่ $Explicit"
    }

    $cmd = Get-Command 'ISCC.exe' -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $candidates = @(
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe')
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }

    if ($candidates) { return $candidates[0] }

    throw 'ไม่พบ Inno Setup 6 — ดาวน์โหลดได้ที่ https://jrsoftware.org/isdl.php แล้วติดตั้งใหม่'
}

$isccPath = Find-Iscc -Explicit $Iscc
Write-Host "ใช้ Inno Setup: $isccPath"

if (-not $SkipBuild) {
    Write-Host '--- cargo build --release ---'
    & cargo build --release --manifest-path (Join-Path $root 'Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw "cargo build ล้มเหลว (exit=$LASTEXITCODE)" }
}

$exe = Join-Path $root 'target\release\mackeys.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw "ไม่พบ $exe" }

$icoPath = Join-Path $root 'assets\mackeys.ico'
Write-Host "--- สร้างไอคอน $icoPath ---"
# ใช้ Start-Process -Wait เพราะตัวโปรแกรมเป็น GUI subsystem (PowerShell จะไม่รอเอง)
$proc = Start-Process -FilePath $exe -ArgumentList 'icon-export', $icoPath -Wait -PassThru -NoNewWindow
if ($proc.ExitCode -ne 0) { throw "สร้างไอคอนล้มเหลว (exit=$($proc.ExitCode))" }

Write-Host '--- คอมไพล์ installer ---'
Push-Location $root
try {
    & $isccPath 'installer\mackeys.iss'
    if ($LASTEXITCODE -ne 0) { throw "ISCC ล้มเหลว (exit=$LASTEXITCODE)" }
} finally {
    Pop-Location
}

$setup = Get-ChildItem (Join-Path $root 'dist') -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($setup) {
    Write-Host ("ได้ไฟล์ installer: {0}  ({1} MB)" -f $setup.FullName, [math]::Round($setup.Length / 1MB, 2))
} else {
    Write-Host 'ไม่พบไฟล์ installer ในโฟลเดอร์ dist\'
}
