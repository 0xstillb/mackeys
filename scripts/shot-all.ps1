# ถ่ายภาพหน้าต่าง MacKeys ทุกหน้า (สำหรับตรวจงาน UI)
# ใช้: powershell -ExecutionPolicy Bypass -File .\scripts\shot-all.ps1 -Prefix "$env:TEMP\mkui"
param(
    [string]$Prefix = "$env:TEMP\mkui"
)

$ErrorActionPreference = 'Stop'
$exe = 'E:\projects\langswitch\target\release\mackeys.exe'

foreach ($page in @('overview', 'shortcuts', 'languages', 'system')) {
    Get-Process -Name 'mackeys' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 700

    $env:MACKEYS_SHOW_UI = '1'
    $env:MACKEYS_PAGE = $page
    Start-Process -FilePath $exe -WorkingDirectory 'E:\projects\langswitch'
    Start-Sleep -Seconds 4

    $out = "$Prefix-$page.png"
    powershell -ExecutionPolicy Bypass -File 'E:\projects\langswitch\scripts\shot.ps1' -Out $out
}

Get-Process -Name 'mackeys' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Output 'เสร็จแล้ว'
