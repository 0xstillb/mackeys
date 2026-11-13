# ตรวจหน้าต่างทั้งหมดของ process (หา hwnd ที่ถูกต้องสำหรับถ่ายภาพ)
param([string]$Name = 'mackeys')

Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class Enum2 {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern int GetWindowTextW(IntPtr hWnd, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }

  public static List<string> List(uint targetPid) {
    var list = new List<string>();
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == targetPid) {
        var sb = new StringBuilder(256);
        GetWindowTextW(h, sb, 256);
        RECT r; GetWindowRect(h, out r);
        list.Add(string.Format("hwnd=0x{0:X} visible={1} rect=({2},{3})-({4},{5}) size={6}x{7} title='{8}'",
          (long)h, IsWindowVisible(h), r.Left, r.Top, r.Right, r.Bottom, r.Right-r.Left, r.Bottom-r.Top, sb.ToString()));
      }
      return true;
    }, IntPtr.Zero);
    return list;
  }
}
"@

$p = Get-Process -Name $Name -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $p) { "ไม่พบ process $Name"; exit 1 }
"pid=$($p.Id) MainWindowHandle=0x$('{0:X}' -f $p.MainWindowHandle.ToInt64())"
foreach ($l in [Enum2]::List([uint32]$p.Id)) { "  $l" }

Add-Type -AssemblyName System.Windows.Forms
$fg = [Enum2]::GetForegroundWindow()
$fgPid = 0
[void][Enum2]::GetWindowThreadProcessId($fg, [ref]$fgPid)
"foreground hwnd=0x$('{0:X}' -f $fg.ToInt64()) pid=$fgPid name=$((Get-Process -Id $fgPid -ErrorAction SilentlyContinue).ProcessName)"
$scr = [System.Windows.Forms.SystemInformation]::VirtualScreen
"virtual screen: $($scr.Left),$($scr.Top) $($scr.Width)x$($scr.Height)"
