use std::ffi::c_void;

use anyhow::{bail, Context, Result};
use image::RgbaImage;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HGDIOBJ, ReleaseDC, SelectObject,
    SRCCOPY,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN,
};

/// จับภาพทั้งจอ (รวมทุกจอถ้าเป็น multi-monitor) → บันทึกไฟล์ PNG + คัดลอกลง clipboard
pub fn capture_fullscreen() -> Result<std::path::PathBuf> {
    let image = grab_virtual_screen()?;
    let path = save_to_pictures(&image)?;

    // คัดลอกลง clipboard — arboard บน Windows ต้องการ **raw RGBA bytes**
    // (ไม่ใช่ไฟล์ PNG/BMP) เพราะมันเขียนลง clipboard เป็น DIB โดยตรง
    let data = arboard::ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: std::borrow::Cow::Owned(image.as_raw().clone()),
    };
    match arboard::Clipboard::new().and_then(|mut c| c.set_image(data)) {
        Ok(()) => tracing::info!("คัดลอกภาพลง clipboard แล้ว"),
        Err(e) => tracing::warn!("คัดลอกลง clipboard ไม่สำเร็จ: {e}"),
    }

    Ok(path)
}

/// จับภาพเฉพาะบริเวณ — ส่งต่อให้ Snipping Tool ของ Windows ทำงาน
pub fn capture_region() -> Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(150));
    crate::win::inject::win_shift_s();
    Ok(())
}

/// เปิดหน้าต่าง Snipping Tool
pub fn open_panel() -> Result<()> {
    crate::win::util::shell_open("ms-screensketch:")
}

fn grab_virtual_screen() -> Result<RgbaImage> {
    unsafe {
        let hdc_screen = GetDC(None);
        if hdc_screen.0.is_null() {
            bail!("เปิด device context ของหน้าจอไม่ได้");
        }

        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        if w <= 0 || h <= 0 {
            let _ = ReleaseDC(None, hdc_screen);
            bail!("ไม่พบข้อมูลขนาดหน้าจอ");
        }

        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbm = CreateCompatibleBitmap(hdc_screen, w, h);
        let old = SelectObject(hdc_mem, HGDIOBJ(hbm.0));

        let blt = BitBlt(hdc_mem, 0, 0, w, h, Some(hdc_screen), x, y, SRCCOPY);
        if blt.is_err() {
            SelectObject(hdc_mem, old);
            let _ = DeleteObject(HGDIOBJ(hbm.0));
            let _ = DeleteDC(hdc_mem);
            let _ = ReleaseDC(None, hdc_screen);
            bail!("คัดลอกภาพหน้าจอไม่สำเร็จ");
        }

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
        let lines = GetDIBits(
            hdc_mem,
            hbm,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // เก็บกวาด
        SelectObject(hdc_mem, old);
        let _ = DeleteObject(HGDIOBJ(hbm.0));
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);

        if lines != h {
            bail!("อ่านข้อมูลภาพไม่ครบ (ได้ {lines} บรรทัด จาก {h})");
        }

        // GDI ให้สีเป็น BGR — สลับให้เป็น RGBA ตามที่ image crate ต้องการ
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
        }

        RgbaImage::from_raw(w as u32, h as u32, buf).ok_or_else(|| anyhow::anyhow!("สร้างภาพไม่สำเร็จ"))
    }
}

fn save_to_pictures(image: &RgbaImage) -> Result<std::path::PathBuf> {
    let base = dirs::picture_dir()
        .or_else(dirs::desktop_dir)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("Screenshots");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("สร้างโฟลเดอร์ {} ไม่ได้", dir.display()))?;

    let path = dir.join(format!("Screenshot {}.png", timestamp()));
    image
        .save(&path)
        .with_context(|| format!("บันทึกภาพไม่ได้: {}", path.display()))?;
    Ok(path)
}

/// เวลาปัจจุบันในรูปแบบที่ใช้เป็นชื่อไฟล์
fn timestamp() -> String {
    unsafe {
        let st = GetLocalTime();
        format!(
            "{:04}-{:02}-{:02} {:02}{:02}{:02}",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
        )
    }
}
