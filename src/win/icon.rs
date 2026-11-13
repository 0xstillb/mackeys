use std::ffi::c_void;

use anyhow::{bail, Result};
use windows::core::BOOL;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS, DeleteObject, GetDC,
    HBITMAP, HDC, HGDIOBJ, ReleaseDC,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};

const SIZE: i32 = 32;
const W: usize = SIZE as usize;

/// สีแบรนด์ #4C6EF5 (RGB)
const BRAND: (u8, u8, u8) = (0x4C, 0x6E, 0xF5);
const WHITE: (u8, u8, u8) = (0xFF, 0xFF, 0xFF);

/// ตำแหน่งปุ่มคีย์บอร์ด 3x3 บนไอคอน 32x32 (มุมบนซ้ายของแต่ละปุ่ม)
const KEYS: [(i32, i32); 9] = [
    (7, 7),
    (13, 7),
    (19, 7),
    (7, 13),
    (13, 13),
    (19, 13),
    (7, 19),
    (13, 19),
    (19, 19),
];

/// ไอคอนที่สร้างไว้ + bitmap ที่ต้องเก็บไว้ให้อายุเท่ากับไอคอน
pub struct AppIcon {
    pub icon: HICON,
    bitmaps: Vec<HBITMAP>,
}

/// ทดสอบว่าพิกเซลอยู่ในสี่เหลี่ยมมนสี่เหลี่ยมหรือไม่
fn in_round_rect(px: f32, py: f32, l: f32, t: f32, r: f32, b: f32, rad: f32) -> bool {
    if px < l || px >= r || py < t || py >= b {
        return false;
    }
    let cx = px.clamp(l + rad, r - rad);
    let cy = py.clamp(t + rad, b - rad);
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= rad * rad
}

/// สร้างพิกเซล RGBA ของไอคอน (กล่องสีมน + ปุ่มคีย์บอร์ดสีขาว)
fn build_rgba() -> Vec<u8> {
    let mut px = vec![0u8; W * W * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let fx = x as f32;
            let fy = y as f32;
            let idx = (y as usize * W + x as usize) * 4;

            // กล่องหลังแบบมนสี่เหลี่ยม
            if !in_round_rect(fx, fy, 3.0, 3.0, 29.0, 29.0, 7.0) {
                continue;
            }

            // ปุ่มคีย์บอร์ดสีขาวทับอยู่บนกล่อง
            let on_key = KEYS.iter().any(|&(kx, ky)| {
                in_round_rect(fx, fy, kx as f32, ky as f32, (kx + 5) as f32, (ky + 5) as f32, 1.0)
            });

            let (r, g, b) = if on_key { WHITE } else { BRAND };
            px[idx] = r;
            px[idx + 1] = g;
            px[idx + 2] = b;
            px[idx + 3] = 255;
        }
    }
    px
}

/// สร้าง HBITMAP 32bpp จากข้อมูล RGBA (แปลงเป็น BGRA ตามที่ GDI ต้องการ)
fn color_bitmap(hdc: HDC, rgba: &[u8]) -> Option<HBITMAP> {
    unsafe {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: SIZE,
                biHeight: -SIZE,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let hbm =
            CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None::<HANDLE>, 0).ok()?;
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ(hbm.0));
            return None;
        }

        let dst = std::slice::from_raw_parts_mut(bits as *mut u8, W * W * 4);
        for (p, src) in dst.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
            p[0] = src[2]; // B
            p[1] = src[1]; // G
            p[2] = src[0]; // R
            p[3] = src[3]; // A
        }
        Some(hbm)
    }
}

/// สร้าง HBITMAP mask 1bpp (ขาว = ทึบแสง) จากช่อง alpha
fn mask_bitmap(hdc: HDC, rgba: &[u8]) -> Option<HBITMAP> {
    unsafe {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: SIZE,
                biHeight: -SIZE,
                biPlanes: 1,
                biBitCount: 1,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let hbm =
            CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None::<HANDLE>, 0).ok()?;
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ(hbm.0));
            return None;
        }

        // 1bpp: แต่ละแถวใช้ 4 ไบต์ (32 bit ปัดให้เป็น 4 ไบต์)
        let stride = ((SIZE as usize + 31) / 32) * 4;
        let dst = std::slice::from_raw_parts_mut(bits as *mut u8, stride * SIZE as usize);
        for y in 0..SIZE as usize {
            for x in 0..W {
                let a = rgba[(y * W + x) * 4 + 3];
                if a > 0 {
                    dst[y * stride + x / 8] |= 0x80 >> (x % 8);
                }
            }
        }
        Some(hbm)
    }
}

/// สร้างไอคอนสำหรับ system tray ตอนรัน (ไม่ต้องพึ่งไฟล์ .ico)
pub fn create_app_icon() -> Option<AppIcon> {
    unsafe {
        let hdc = GetDC(None);
        if hdc.0.is_null() {
            return None;
        }
        let rgba = build_rgba();
        let hbm_color = color_bitmap(hdc, &rgba)?;
        let hbm_mask = mask_bitmap(hdc, &rgba)?;

        let icon_info = ICONINFO {
            fIcon: BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };
        let icon = CreateIconIndirect(&icon_info).ok()?;
        let _ = ReleaseDC(None, hdc);

        Some(AppIcon {
            icon,
            bitmaps: vec![hbm_color, hbm_mask],
        })
    }
}

impl Drop for AppIcon {
    fn drop(&mut self) {
        unsafe {
            for bmp in &self.bitmaps {
                let _ = DeleteObject(HGDIOBJ(bmp.0));
            }
        }
    }
}

/// วาดไอคอนเป็นไฟล์ PNG (ใช้ตรวจว่าไอคอนวาดออกมาถูกต้อง)
pub fn preview_png(path: &std::path::Path) -> Result<()> {
    let rgba = build_rgba();
    let img = image::RgbaImage::from_raw(SIZE as u32, SIZE as u32, rgba)
        .ok_or_else(|| anyhow::anyhow!("สร้างภาพไอคอนไม่สำเร็จ"))?;

    // ขยาย 8x เพื่อให้ตรวจสอบรายละเอียดได้ง่าย
    let big = image::imageops::resize(
        &img,
        SIZE as u32 * 8,
        SIZE as u32 * 8,
        image::imageops::FilterType::Nearest,
    );
    big.save(path)?;
    if !path.exists() {
        bail!("เขียนไฟล์ {} ไม่สำเร็จ", path.display());
    }
    Ok(())
}
