# ทดสอบ ⌘+Tab แบบ macOS: กด ⌘ ค้าง → task switcher ต้องเปิดค้างไว้
# แล้วกด Tab ซ้ำได้ (เลื่อนตัวที่เลือก) ปล่อย ⌘ ค่อย commit
param(
    [int]$TabPresses = 3
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

public class SimSwitcher {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern uint SendInput(uint n, INPUT[] p, int size);
    [DllImport("user32.dll")]
    public static extern short GetAsyncKeyState(int vKey);

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

    public static string AltState() {
        return ((GetAsyncKeyState(0xA4) & 0x8000) != 0) ? "Alt ถูกกดอยู่" : "Alt ไม่ถูกกด";
    }
}
"@

$VK_LMENU = 0xA4
$VK_TAB = 0x09

Write-Output "ก่อนกด: $([SimSwitcher]::AltState())"
[SimSwitcher]::Key($VK_LMENU, $false)   # กด ⌘
Start-Sleep -Milliseconds 150
Write-Output "หลังกด ⌘: $([SimSwitcher]::AltState())  (ต้องเป็น 'Alt ไม่ถูกกด' เพราะเรากลืน Alt)"

for ($i = 1; $i -le $TabPresses; $i++) {
    [SimSwitcher]::Key($VK_TAB, $false)
    Start-Sleep -Milliseconds 120
    [SimSwitcher]::Key($VK_TAB, $true)
    Start-Sleep -Milliseconds 250
    Write-Output "หลังกด Tab ครั้งที่ $i : $([SimSwitcher]::AltState())  (ต้องเป็น 'Alt ถูกกดอยู่' = switcher ยังเปิดค้าง)"
}

[SimSwitcher]::Key($VK_LMENU, $true)    # ปล่อย ⌘
Start-Sleep -Milliseconds 300
Write-Output "หลังปล่อย ⌘: $([SimSwitcher]::AltState())  (ต้องเป็น 'Alt ไม่ถูกกด' = ปิด switcher แล้ว)"
