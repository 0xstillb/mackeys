# ทดสอบจริงว่า "แก้คีย์ลัด" ใช้ได้จริงไหม
# คลิกช่องกรอก → พิมพ์ → ดูผลตรวจสอบสด → กด "ตกลง" → กด "บันทึก" → ตรวจ config.toml ว่าเปลี่ยนจริง
# แล้วคืนค่า config เดิมเสมอ (ไม่ทิ้งไฟล์เสียหาย)
param(
    [string]$NewKeys = 'ctrl+shift+9',
    [string]$BadKeys = 'ctrl+shift+zz',
    [int]$EditIndex = 1,
    [string]$ShotPrefix = "$env:TEMP\mkedit"
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Edit {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint flags);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern uint SendInput(uint n, INPUT[] p, int size);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, IntPtr pid);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool f);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetActiveWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);

  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }

  [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT {
    public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo;
  }
  [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT {
    public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo;
  }
  [StructLayout(LayoutKind.Explicit)] public struct INPUTUNION {
    [FieldOffset(0)] public KEYBDINPUT ki;
    [FieldOffset(0)] public MOUSEINPUT mi;
  }
  [StructLayout(LayoutKind.Sequential)] public struct INPUT {
    public uint type; public INPUTUNION u;
  }

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

  public static void ForceTop(IntPtr hWnd) {
    IntPtr TOPMOST = new IntPtr(-1); IntPtr NOTOPMOST = new IntPtr(-2);
    SetWindowPos(hWnd, TOPMOST, 0, 0, 0, 0, 0x0002 | 0x0001 | 0x0040 | 0x0010);
    SetWindowPos(hWnd, NOTOPMOST, 0, 0, 0, 0, 0x0002 | 0x0001 | 0x0040 | 0x0010);
    ShowWindow(hWnd, 9); BringWindowToTop(hWnd);
  }

  public static bool ForceForeground(IntPtr hWnd) {
    IntPtr fg = GetForegroundWindow();
    uint fgThread = GetWindowThreadProcessId(fg, IntPtr.Zero);
    uint myThread = GetCurrentThreadId();
    bool attached = false;
    if (fgThread != 0 && fgThread != myThread) attached = AttachThreadInput(myThread, fgThread, true);
    ShowWindow(hWnd, 9); BringWindowToTop(hWnd);
    SetForegroundWindow(hWnd); SetActiveWindow(hWnd);
    if (attached) AttachThreadInput(myThread, fgThread, false);
    return GetForegroundWindow() == hWnd;
  }

  // พิมพ์ข้อความจริง (KEYEVENTF_UNICODE)
  public static void TypeText(string s) {
    foreach (char c in s) {
      INPUT[] inp = new INPUT[2];
      inp[0].type = 1; inp[0].u.ki.wScan = (ushort)c; inp[0].u.ki.dwFlags = 0x0004;
      inp[1].type = 1; inp[1].u.ki.wScan = (ushort)c; inp[1].u.ki.dwFlags = 0x0004 | 0x0002;
      SendInput(2, inp, Marshal.SizeOf(typeof(INPUT)));
      System.Threading.Thread.Sleep(12);
    }
  }

  // กดปุ่ม (vk) จำนวนครั้ง
  public static void PressKey(ushort vk, int times) {
    for (int i = 0; i < times; i++) {
      INPUT[] inp = new INPUT[2];
      inp[0].type = 1; inp[0].u.ki.wVk = vk;
      inp[1].type = 1; inp[1].u.ki.wVk = vk; inp[1].u.ki.dwFlags = 0x0002;
      SendInput(2, inp, Marshal.SizeOf(typeof(INPUT)));
      System.Threading.Thread.Sleep(12);
    }
  }

  // คลิกที่พิกัด client (x,y) ของหน้าต่าง
  public static void ClickClient(IntPtr hWnd, int cx, int cy) {
    POINT p; p.X = cx; p.Y = cy;
    ClientToScreen(hWnd, ref p);
    SetCursorPos(p.X, p.Y);
    System.Threading.Thread.Sleep(120);
    mouse_event(0x0002, 0, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(60);
    mouse_event(0x0004, 0, 0, 0, IntPtr.Zero);
  }
}
"@

$exe = 'E:\projects\langswitch\target\release\mackeys.exe'
$cfg = "$env:APPDATA\MacKeys\config.toml"

# ── 1. สำรอง config เดิม (จะคืนค่าเสมอ) ────────────────────────────
$backup = "$env:TEMP\mackeys-config-backup.toml"
Copy-Item $cfg $backup -Force
Write-Output "สำรอง config ไว้ที่: $backup"

function Restore-Config {
    Copy-Item $backup $cfg -Force
    Write-Output "คืนค่า config เดิมแล้ว"
}

try {
    # ── 2. เปิดโปรแกรมในโหมดแก้ไขคีย์ลัด ───────────────────────────
    Get-Process -Name 'mackeys' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 700
    $env:MACKEYS_SHOW_UI = '1'
    $env:MACKEYS_PAGE = 'shortcuts'
    $env:MACKEYS_EDIT = "$EditIndex"
    Start-Process -FilePath $exe -WorkingDirectory 'E:\projects\langswitch'
    Start-Sleep -Seconds 5

    $proc = Get-Process -Name 'mackeys' -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $proc) { throw 'โปรแกรมไม่ได้สตาร์ท' }
    $hwnd = [Edit]::BiggestWindow([uint32]$proc.Id)
    if ($hwnd -eq [IntPtr]::Zero) { throw 'ไม่พบหน้าต่าง' }

    [void][Edit]::ForceTop($hwnd)
    if (-not [Edit]::ForceForeground($hwnd)) { throw 'ดึงหน้าต่างมา foreground ไม่ได้' }
    Start-Sleep -Milliseconds 800

    function Save-Shot([string]$name) {
        $r = New-Object Edit+RECT
        [void][Edit]::GetWindowRect($hwnd, [ref]$r)
        $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
        $bmp = New-Object System.Drawing.Bitmap $w, $h
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $hdc = $g.GetHdc()
        [void][Edit]::PrintWindow($hwnd, $hdc, 2)
        $g.ReleaseHdc($hdc)
        $out = "$ShotPrefix-$name.png"
        $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
        $g.Dispose(); $bmp.Dispose()
        Write-Output "  ภาพ: $out"
    }

    # พิกัดในพื้นที่ client (วัดจากภาพจริงของหน้าต่าง 900x640)
    $FIELD_X = 382; $FIELD_Y = 383     # ช่องกรอกคีย์ลัด
    $OK_X = 294; $OK_Y = 453           # ปุ่ม "ตกลง"
    $SAVE_X = 827; $SAVE_Y = 30        # ปุ่ม "บันทึก" ในแถบบน

    Write-Output '--- ขั้นที่ 1: คลิกช่องกรอก + ลบข้อความเดิม + พิมพ์ค่าใหม่ ---'
    # คลิก 2 ครั้ง: คลิกแรกของ Windows ไปติดการ "เปิดใช้งานหน้าต่าง" (activate) ไม่ใช่การโฟกัส
    [Edit]::ClickClient($hwnd, $FIELD_X, $FIELD_Y)
    Start-Sleep -Milliseconds 500
    [Edit]::ClickClient($hwnd, $FIELD_X, $FIELD_Y)
    Start-Sleep -Milliseconds 500
    [void][Edit]::ForceForeground($hwnd)
    Start-Sleep -Milliseconds 300
    [Edit]::PressKey(0x23, 1)          # End
    Start-Sleep -Milliseconds 200
    [Edit]::PressKey(0x08, 24)         # Backspace x24
    Start-Sleep -Milliseconds 300
    [Edit]::TypeText($NewKeys)
    Start-Sleep -Milliseconds 900
    Save-Shot '1-typed-valid'

    Write-Output '--- ขั้นที่ 2: พิมพ์ค่าที่ผิด ดูว่าขึ้นสถานะแดงไหม ---'
    [Edit]::PressKey(0x23, 1)
    Start-Sleep -Milliseconds 150
    [Edit]::PressKey(0x08, 24)
    Start-Sleep -Milliseconds 200
    [Edit]::TypeText($BadKeys)
    Start-Sleep -Milliseconds 900
    Save-Shot '2-typed-invalid'

    Write-Output '--- ขั้นที่ 3: พิมพ์ค่าที่ถูกต้องกลับ + กด "ตกลง" ---'
    [Edit]::PressKey(0x23, 1)
    Start-Sleep -Milliseconds 150
    [Edit]::PressKey(0x08, 24)
    Start-Sleep -Milliseconds 200
    [Edit]::TypeText($NewKeys)
    Start-Sleep -Milliseconds 600
    [Edit]::ClickClient($hwnd, $OK_X, $OK_Y)
    Start-Sleep -Milliseconds 900
    Save-Shot '3-clicked-ok'

    Write-Output '--- ขั้นที่ 4: กด "บันทึก" ในแถบบน แล้วอ่าน config.toml ---'
    # ปุ่ม "บันทึก" ขยับตำแหน่งตามป้าย "มีการเปลี่ยนแปลง" → หาจากสีในภาพจริงแทน
    $shot = "$ShotPrefix-3-clicked-ok.png"
    $b = New-Object System.Drawing.Bitmap $shot
    $sx = @(); $sy = @()
    for ($y = 25; $y -lt 70; $y++) {
        for ($x = 600; $x -lt $b.Width; $x++) {
            $c = $b.GetPixel($x, $y)
            if ([math]::Abs($c.R - 108) -lt 14 -and [math]::Abs($c.G - 140) -lt 14 -and [math]::Abs($c.B - 255) -lt 14) {
                $sx += $x; $sy += $y
            }
        }
    }
    $b.Dispose()
    if ($sx.Count -lt 50) { throw 'หาปุ่ม "บันทึก" (สีหลัก) ในแถบบนไม่เจอ' }
    $btnClientX = [int](($sx | Measure-Object -Average).Average) - 8
    $btnClientY = [int](($sy | Measure-Object -Average).Average) - 31
    Write-Output "  ปุ่ม " + '"บันทึก"' + " อยู่ที่ client ($btnClientX,$btnClientY) — คลิกแล้ว"
    [Edit]::ClickClient($hwnd, $btnClientX, $btnClientY)
    Start-Sleep -Seconds 2
    Save-Shot '4-saved'

    $content = Get-Content $cfg -Raw
    $found = $content -match ('keys\s*=\s*"' + [regex]::Escape($NewKeys) + '"')
    Write-Output ("config.toml มีค่า {0} : {1}" -f $NewKeys, $found)
    if (-not $found) {
        Write-Output '--- เนื้อหา config.toml ปัจจุบัน ---'
        Write-Output $content
        throw 'บันทึกไม่สำเร็จ: config.toml ไม่มีค่าที่แก้'
    }
    Write-Output 'ผ่าน: แก้คีย์ลัดแล้วบันทึกลงไฟล์จริง'
}
finally {
    Get-Process -Name 'mackeys' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Restore-Config
    Start-Sleep -Milliseconds 500
    # เปิดกลับเป็นโหมด tray ตามปกติ
    $env:MACKEYS_SHOW_UI = $null
    $env:MACKEYS_PAGE = $null
    $env:MACKEYS_EDIT = $null
    Start-Process -FilePath $exe -WorkingDirectory 'E:\projects\langswitch'
    Write-Output 'เปิดโปรแกรมกลับเป็นโหมด tray แล้ว'
}
