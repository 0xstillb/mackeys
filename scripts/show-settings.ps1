# เปิดหน้าต่างตั้งค่าของ MacKeys "แบบ production" (ส่ง WM_COMMAND ไปที่ window ของ runner)
#
# ทำไมต้องทำแบบนี้: ถ้าเปิดด้วย MACKEYS_SHOW_UI=1 (สร้างหน้าต่างตั้งแต่แรกแบบซ่อนไว้)
# พื้นผิว GPU จะไม่ถูกสร้าง → หน้าต่างขาวโพลน และถ่ายภาพไม่ได้
# วิธีนี้จำลองสิ่งที่ผู้ใช้ทำจริง (คลิกไอคอนใน tray) → หน้าต่างแสดงผลถูกต้อง
param(
    [string]$Page = ''
)

Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class W4 {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc f, IntPtr l);
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr h, uint msg, IntPtr wp, IntPtr lp);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }

  public static IntPtr HostWindow(uint targetPid) {
    IntPtr best = IntPtr.Zero; long bestArea = 0;
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == targetPid) {
        RECT r; GetWindowRect(h, out r);
        long area = (long)(r.Right - r.Left) * (r.Bottom - r.Top);
        // หน้าต่างของ runner คืออันที่เล็กที่สุด (ซ่อนอยู่) — ไม่ใช่หน้าต่าง egui
        if (area > 0 && area < bestArea == false && area < 2000) { best = h; }
        if (area > 0 && area < 2000) { best = h; }
      }
      return true;
    }, IntPtr.Zero);
    return best;
  }
}
"@

$p = Get-Process -Name mackeys -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output "ไม่พบ process mackeys"; exit 1 }

$host_hwnd = [W4]::HostWindow([uint32]$p.Id)
if ($host_hwnd -eq [IntPtr]::Zero) { Write-Output "ไม่พบ window ของ runner"; exit 1 }
Write-Output "runner hwnd=0x$('{0:X}' -f $host_hwnd.ToInt64())"

# WM_COMMAND (0x0111) พร้อม wParam = id คำสั่ง 1002 (เปิดหน้าต่างตั้งค่า)
[void][W4]::SendMessageW($host_hwnd, 0x0111, [IntPtr]1002, [IntPtr]::Zero)
Start-Sleep -Seconds 2
Write-Output "ส่งคำสั่งเปิดหน้าต่างตั้งค่าแล้ว"
