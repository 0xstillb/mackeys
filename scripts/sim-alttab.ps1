# จำลองการกด Alt+Tab (⌘+Tab ในโหมด Mac) เพื่อทดสอบ app switcher
# ใช้ตอนทดสอบโหมด Mac โดยตั้ง MACKEYS_TEST_INJECTED=1
param(
    [int]$Times = 3,
    [int]$GapMs = 700
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

public class SimTab {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern uint SendInput(uint n, INPUT[] p, int size);

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
$VK_TAB = 0x09

for ($i = 0; $i -lt $Times; $i++) {
    [SimTab]::Key($VK_LMENU, $false)   # กด Alt ซ้าย (= ⌘)
    Start-Sleep -Milliseconds 80
    [SimTab]::Key($VK_TAB, $false)     # กด Tab
    Start-Sleep -Milliseconds 80
    [SimTab]::Key($VK_TAB, $true)      # ปล่อย Tab
    Start-Sleep -Milliseconds 80
    [SimTab]::Key($VK_LMENU, $true)    # ปล่อย Alt
    Start-Sleep -Milliseconds $GapMs
}
Write-Output "done $Times times"
