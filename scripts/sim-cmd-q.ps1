# ทดสอบ ⌘+Q (ปิดแอปที่โฟกัส) แบบอัตโนมัติ
#
# ใช้ตอนทดสอบโหมด Mac โดยตั้ง MACKEYS_TEST_INJECTED=1 ตอน start โปรแกรม
#
# ขั้นตอน:
#   1. เปิดแอปเป้าหมาย (default: Notepad)
#   2. ยืนยันว่า foreground เป็นแอปเป้าหมายจริง (กันไม่ให้ยิง Alt+F4 ไปโดนหน้าต่างอื่น)
#   3. ส่ง Alt ซ้ายลง (= ⌘) แล้วกด Q
#   4. รอ แล้วดูว่าแอปเป้าหมายถูกปิดหรือยัง
param(
    [string]$Target = 'Notepad',
    [int]$WaitMs = 1500
)

Add-Type @"
using System;
using System.Runtime.InteropServices;

public struct KEYBDINPUT {
    public ushort wVk;
    public ushort wScan;
    public uint dwFlags;
    public uint time;
    public IntPtr dwExtraInfo;
}

[StructLayout(LayoutKind.Sequential)]
public struct MOUSEINPUT {
    public int dx;
    public int dy;
    public uint mouseData;
    public uint dwFlags;
    public uint time;
    public IntPtr dwExtraInfo;
}

[StructLayout(LayoutKind.Explicit)]
public struct INPUTUNION {
    [FieldOffset(0)] public KEYBDINPUT ki;
    [FieldOffset(0)] public MOUSEINPUT mi;
}

[StructLayout(LayoutKind.Sequential)]
public struct INPUT {
    public uint type;
    public INPUTUNION u;
}

public class Sim2 {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern uint SendInput(uint n, INPUT[] p, int size);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    [DllImport("user32.dll")]
    public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool fAttach);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, IntPtr pid);

    [DllImport("kernel32.dll")]
    public static extern uint GetCurrentThreadId();

    [DllImport("user32.dll")]
    public static extern bool BringWindowToTop(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool SetActiveWindow(IntPtr hWnd);

    // ยืนยันว่าหน้าต่างนี้เป็น foreground จริง
    // (โปรแกรมที่รันเป็น background ไม่มีสิทธิ์ SetForegroundWindow ตรง ๆ
    //  ต้องใช้ AttachThreadInput ผูก thread เข้ากับ thread ของหน้าต่างที่โฟกัสอยู่)
    public static bool ForceForeground(IntPtr hWnd) {
        IntPtr fg = GetForegroundWindow();
        uint fgThread = GetWindowThreadProcessId(fg, IntPtr.Zero);
        uint myThread = GetCurrentThreadId();
        bool attached = false;
        if (fgThread != 0 && fgThread != myThread) {
            attached = AttachThreadInput(myThread, fgThread, true);
        }
        ShowWindow(hWnd, 9);
        BringWindowToTop(hWnd);
        SetForegroundWindow(hWnd);
        SetActiveWindow(hWnd);
        if (attached) {
            AttachThreadInput(myThread, fgThread, false);
        }
        return GetForegroundWindow() == hWnd;
    }

    public static void Key(ushort vk, bool up) {
        INPUT[] inp = new INPUT[1];
        inp[0].type = 1;
        inp[0].u.ki.wVk = vk;
        inp[0].u.ki.wScan = 0;
        inp[0].u.ki.dwFlags = up ? (uint)2 : (uint)0;
        inp[0].u.ki.time = 0;
        inp[0].u.ki.dwExtraInfo = IntPtr.Zero;
        uint sent = SendInput(1, inp, Marshal.SizeOf(typeof(INPUT)));
        if (sent != 1) { Console.WriteLine("SendInput failed: " + Marshal.GetLastWin32Error()); }
    }
}
"@

$VK_LMENU = 0xA4
$VK_Q     = 0x51

# ── 1. เปิดแอปเป้าหมาย ────────────────────────────────────────────
# ต้องรอจนกว่าแอปจะ "มีหน้าต่างจริง" (MainWindowHandle != 0) ไม่งั้นจะโฟกัสไม่ได้
function Get-TargetWindow {
    param([string]$Name)
    for ($i = 0; $i -lt 30; $i++) {
        $p = Get-Process -Name $Name -ErrorAction SilentlyContinue |
             Where-Object { $_.MainWindowHandle -ne 0 } |
             Select-Object -First 1
        if ($p) { return $p }
        Start-Sleep -Milliseconds 300
    }
    return $null
}

$proc = Get-TargetWindow -Name $Target
if (-not $proc) {
    Start-Process -FilePath "$Target.exe" -ErrorAction SilentlyContinue
    $proc = Get-TargetWindow -Name $Target
}
if (-not $proc) {
    Write-Output "FAIL: เปิด $Target ไม่ได้ (ไม่มีหน้าต่าง)"
    exit 1
}
$pidTarget = $proc.Id
$hwndTarget = $proc.MainWindowHandle
Write-Output "เป้าหมาย: $Target pid=$pidTarget hwnd=$hwndTarget"

# ── 2. ยืนยันว่า foreground คือแอปเป้าหมายจริง ───────────────────
[void][Sim2]::ForceForeground($hwndTarget)
Start-Sleep -Milliseconds 700

$fg = [Sim2]::GetForegroundWindow()
$fgPid = 0
[void][Sim2]::GetWindowThreadProcessId($fg, [ref]$fgPid)
if ($fgPid -ne $pidTarget) {
    Write-Output "FAIL: foreground ไม่ใช่แอปเป้าหมาย (fg pid=$fgPid) — ยกเลิกเพื่อไม่ให้ยิงผิดหน้าต่าง"
    exit 2
}
Write-Output "foreground ตรงเป้าหมาย — เริ่มส่ง ⌘+Q"

# ── 3. ส่ง ⌘+Q ─────────────────────────────────────────────────────
[Sim2]::Key($VK_LMENU, $false)    # กด Alt ซ้าย (= ⌘)
Start-Sleep -Milliseconds 80
[Sim2]::Key($VK_Q, $false)        # กด Q
Start-Sleep -Milliseconds 80
[Sim2]::Key($VK_Q, $true)         # ปล่อย Q
Start-Sleep -Milliseconds 120
[Sim2]::Key($VK_LMENU, $true)     # ปล่อย Alt
Write-Output "ส่ง ⌘+Q แล้ว รอ $WaitMs ms"

# ── 4. ผลลัพธ์ ────────────────────────────────────────────────────
Start-Sleep -Milliseconds $WaitMs
$still = Get-Process -Id $pidTarget -ErrorAction SilentlyContinue
if ($still) {
    Write-Output "RESULT: แอปยังเปิดอยู่ -> ⌘Q ไม่ได้ปิดแอป"
    exit 3
} else {
    Write-Output "RESULT: แอปถูกปิด -> ⌘Q ทำงานถูกต้อง"
    exit 0
}
