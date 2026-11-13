# จับภาพหน้าต่าง MacKeys สำหรับตรวจงาน UI
# ใช้: powershell -ExecutionPolicy Bypass -File .\scripts\shot.ps1 -Out "$env:TEMP\mackeys.png"
param(
    [string]$Out = "$env:TEMP\mackeys-ui.png",
    [string]$WindowTitle = "MacKeys"
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  // วาดเนื้อหาหน้าต่างลง bitmap โดยตรง (ไม่สนใจว่ามีหน้าต่างอื่นทับอยู่ไหม)
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }

  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, IntPtr pid);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool f);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetActiveWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int x, int y, int cx, int cy, uint flags);

  // เลือกหน้าต่างที่ใหญ่ที่สุดของ process (MainWindowHandle อาจชี้ไปที่ไอคอนใน tray ซึ่งเป็น 16x16)
  public static IntPtr BiggestWindow(uint targetPid) {
    IntPtr best = IntPtr.Zero; long bestArea = 0;
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == targetPid) {
        RECT r; GetWindowRect(h, out r);
        long area = (long)(r.Right - r.Left) * (r.Bottom - r.Top);
        if (area > bestArea) { bestArea = area; best = h; }
      }
      return true;
    }, IntPtr.Zero);
    return best;
  }

  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);

  // ยกหน้าต่างขึ้นหน้าสุดแบบ "ไม่มีเงื่อนไข" — ใช้ตอนโปรแกรมเราทำงานเป็น background
  // (SetForegroundWindow ตรง ๆ จะถูก Windows ปฏิเสธถ้าไม่ได้รับ input ล่าสุด)
  public static void ForceTop(IntPtr hWnd) {
    IntPtr TOPMOST = new IntPtr(-1);
    IntPtr NOTOPMOST = new IntPtr(-2);
    const uint SWP_NOMOVE = 0x0002;
    const uint SWP_NOSIZE = 0x0001;
    const uint SWP_SHOWWINDOW = 0x0040;
    const uint SWP_NOACTIVATE = 0x0010;
    SetWindowPos(hWnd, TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE);
    SetWindowPos(hWnd, NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE);
    ShowWindow(hWnd, 9);
    BringWindowToTop(hWnd);
  }

  // บังคับให้หน้าต่างขึ้นหน้าสุด (โปรแกรม background ไม่มีสิทธิ์ SetForegroundWindow ตรง ๆ)
  public static bool ForceForeground(IntPtr hWnd) {
    IntPtr fg = GetForegroundWindow();
    uint fgThread = GetWindowThreadProcessId(fg, IntPtr.Zero);
    uint myThread = GetCurrentThreadId();
    bool attached = false;
    if (fgThread != 0 && fgThread != myThread) { attached = AttachThreadInput(myThread, fgThread, true); }
    ShowWindow(hWnd, 9);
    BringWindowToTop(hWnd);
    SetForegroundWindow(hWnd);
    SetActiveWindow(hWnd);
    if (attached) { AttachThreadInput(myThread, fgThread, false); }
    return GetForegroundWindow() == hWnd;
  }
}
"@

$proc = Get-Process -Name 'mackeys' -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) { throw "ไม่พบ process mackeys" }

$hwnd = [Win]::BiggestWindow([uint32]$proc.Id)
if ($hwnd -eq [IntPtr]::Zero) {
    throw "ไม่พบหน้าต่างของโปรแกรม"
}

[void][Win]::ShowWindow($hwnd, 9)   # SW_RESTORE
[void][Win]::ForceTop($hwnd)
[void][Win]::ForceForeground($hwnd)
Start-Sleep -Milliseconds 1200

$r = New-Object Win+RECT
[void][Win]::GetWindowRect($hwnd, [ref]$r)
$w = $r.Right - $r.Left
$h = $r.Bottom - $r.Top
if ($w -le 0 -or $h -le 0) { throw "อ่านขนาดหน้าต่างไม่ได้" }

$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)

# ── วิธีที่ 1: PrintWindow ─────────────────────────────────────────
# หน้าต่าง egui ใช้ GPU (Direct2D/OpenGL) → PrintWindow มักได้ภาพขาว
# เลยต้องเช็คว่าภาพ "ว่าง" หรือไม่ แล้วค่อยตกไปใช้วิธีถัดไป
$hdc = $g.GetHdc()
$ok = [Win]::PrintWindow($hwnd, $hdc, 2)
$g.ReleaseHdc($hdc)

function Test-Blank([System.Drawing.Bitmap]$b) {
    # ดูพิกเซลกลาง ๆ 5 จุด — ถ้าขาวเกือบทั้งหมดถือว่าจับไม่ได้
    $pts = @(
        @([int]($b.Width * 0.5), [int]($b.Height * 0.3)),
        @([int]($b.Width * 0.2), [int]($b.Height * 0.6)),
        @([int]($b.Width * 0.8), [int]($b.Height * 0.6)),
        @([int]($b.Width * 0.5), [int]($b.Height * 0.8)),
        @([int]($b.Width * 0.35), [int]($b.Height * 0.15))
    )
    foreach ($p in $pts) {
        $c = $b.GetPixel($p[0], $p[1])
        if (($c.R + $c.G + $c.B) -lt 690) { return $false }   # ไม่ขาวเกือบทั้งหมด
    }
    return $true
}

$method = 'PrintWindow'
if (-not $ok -or (Test-Blank $bmp)) {
    # ── วิธีที่ 2: ถ่ายจากหน้าจอ (หน้าต่างถูกยกขึ้นหน้าสุดแล้ว) ──────
    $g.Dispose()
    $bmp.Dispose()
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
    $method = 'CopyFromScreen'
}

$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()

Write-Output "บันทึกแล้ว: $Out ($w x $h, วิธี=$method)"
