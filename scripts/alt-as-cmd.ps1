<#
.SYNOPSIS
    เปลี่ยนกายปุ่ม Alt ฝั่งซ้าย ให้ทำหน้าที่เป็น Ctrl (⌘ ของ macOS) แบบระดับไดรเวอร์

.DESCRIPTION
    ทางเลือกแบบ "ไม่ใช้ hook" ของ MacKeys — แก้ที่ Scancode Map ของ Windows
    ดังนั้นแอปทุกตัว (รวมโปรแกรมที่รันสิทธิ์สูง) จะได้รับ Ctrl จริง ๆ
    ไม่มีความเสี่ยง input lag และเป็นการแก้ครั้งเดียวถาวร

    ผลลัพธ์:
      - ปุ่ม Alt ฝั่งซ้าย  → ทำหน้าที่เป็น Ctrl (เหมือนกด ⌘ บน macOS)
      - ปุ่ม Alt ฝั่งขวา  → ยังเป็น Alt จริง (Alt+Tab / Alt+F4 / เมนู)
      - ปุ่ม Ctrl เดิม     → ยังเป็น Ctrl จริง

    คีย์ลัดของ MacKeys ที่ผูกกับ Ctrl (Ctrl+Space, Ctrl+Tab, Ctrl+Shift+3/4/5)
    จะทำงานทันทีเมื่อผู้ใช้กดปุ่ม Alt ฝั่งซ้าย

.PARAMETER Disable
    ย้อนกลับกลับเป็นปุ่ม Alt ปกติ

.NOTES
    ต้องรันด้วยสิทธิ์ Administrator และ reboot 1 ครั้งเพื่อให้มีผล
    สคริปต์จะสำรองค่าเดิมไว้ที่ %TEMP%\MacKeys-ScancodeMap-backup.reg
#>

[CmdletBinding()]
param(
    [switch]$Disable
)

$ErrorActionPreference = 'Stop'

$regPath = 'HKLM:\SYSTEM\CurrentControlSet\Control\Keyboard Layout'
$valueName = 'Scancode Map'
$backupFile = Join-Path $env:TEMP 'MacKeys-ScancodeMap-backup.reg'

# ตรวจว่าเป็น Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host '❌ ต้องรันด้วยสิทธิ์ Administrator' -ForegroundColor Red
    Write-Host '   ลอง: powershell -ExecutionPolicy Bypass -File .\scripts\alt-as-cmd.ps1' -ForegroundColor Yellow
    exit 1
}

# ── สำรองค่าเดิม (ทำครั้งแรก) ────────────────────────────────────────────
$existing = Get-ItemProperty -Path $regPath -Name $valueName -ErrorAction SilentlyContinue
if ($existing) {
    if (-not (Test-Path $backupFile)) {
        $bytes = $existing.$valueName
        $hex = ($bytes | ForEach-Object { $_.ToString('x2') }) -join ','
        @(
            'Windows Registry Editor Version 5.00'
            ''
            "[$regPath]"
            "`"$valueName`"=hex:$hex"
        ) | Set-Content -Path $backupFile -Encoding ASCII
        Write-Host "💾 สำรองค่าเดิมไว้ที่ $backupFile" -ForegroundColor Cyan
    }
}

if ($Disable) {
    # ลบค่าออกทั้งหมด → ปุ่มกลับเป็นปกติ
    Remove-ItemProperty -Path $regPath -Name $valueName -ErrorAction SilentlyContinue
    Write-Host '✅ ย้อนกลับแล้ว — ปุ่ม Alt กลับเป็น Alt ปกติ (reboot เพื่อให้มีผล)' -ForegroundColor Green
    exit 0
}

# ── ตั้งค่าใหม่: Left Alt (38) → Left Ctrl (1D) ─────────────────────────
# รูปแบบ: header(2 DWORD) + จำนวนรายการ + mapping + terminator
$bytes = [byte[]]@(
    0x00, 0x00, 0x00, 0x00,   # header 1
    0x00, 0x00, 0x00, 0x00,   # header 2
    0x02, 0x00, 0x00, 0x00,   # จำนวนรายการ (mapping 1 ตัว + terminator)
    0x38, 0x00, 0x1D, 0x00,   # Left Alt (0x38) → Left Ctrl (0x1D)
    0x00, 0x00, 0x00, 0x00    # terminator
)

New-ItemProperty -Path $regPath -Name $valueName -Value $bytes `
    -PropertyType Binary -Force | Out-Null

Write-Host '✅ ตั้งค่าแล้ว: ปุ่ม Alt ฝั่งซ้าย → ทำหน้าที่เป็น Ctrl' -ForegroundColor Green
Write-Host ''
Write-Host '   Alt+C        = ⌘C  (copy)' -ForegroundColor Gray
Write-Host '   Alt+V        = ⌘V  (paste)' -ForegroundColor Gray
Write-Host '   Alt+Space    = สลับภาษาอินพุต (ผ่าน MacKeys)' -ForegroundColor Gray
Write-Host '   Alt+Tab      = สลับแอป (ผ่าน MacKeys)' -ForegroundColor Gray
Write-Host '   Alt+Shift+3  = จับภาพทั้งจอ (ผ่าน MacKeys)' -ForegroundColor Gray
Write-Host '   Alt ฝั่งขวา  = ยังเป็น Alt จริง' -ForegroundColor Gray
Write-Host ''
Write-Host '⚠️  ต้อง reboot 1 ครั้งเพื่อให้มีผล' -ForegroundColor Yellow
Write-Host "   ย้อนกลับ: รันสคริปต์นี้อีกครั้งพร้อมพารามิเตอร์ -Disable" -ForegroundColor Yellow
