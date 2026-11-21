use std::ffi::c_void;
use std::fs;
use std::path::Path;

use anyhow::{bail, Result};
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use windows::core::BOOL;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS, DeleteObject, GetDC,
    HBITMAP, HDC, HGDIOBJ, ReleaseDC,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};

const SIZE: i32 = 32;
const W: usize = SIZE as usize;

/// ขนาดภาพที่ใส่ในไฟล์ .ico (Windows ใช้ 0 แทน 256 ในไบต์ขนาดภาพ)
const ICO_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

/// สีแบรนด์ #4C6EF5 (RGB)
const BRAND: (u8, u8, u8) = (0x4C, 0x6E, 0xF5);
const WHITE: (u8, u8, u8) = (0xFF, 0xFF, 0xFF);

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
    build_rgba_at(SIZE as u32)
}

/// แปลงพิกัดจากกริดอ้างอิง 32x32 ไปยังขนาดที่ต้องการ (เลขฐานสองปัดให้ตรงกับกริดเดิมเมื่อ size = 32)
fn scale(v: i32, size: u32) -> i32 {
    let size = size as i32;
    (v * size + 16) / 32
}

/// วาดไอคอนที่ขนาด `size` x `size` เป็น RGBA (วาดสัดส่วนเดียวกันทุกขนาด)
fn build_rgba_at(size: u32) -> Vec<u8> {
    let size = size as i32;
    let mut px = vec![0u8; (size * size * 4) as usize];

    // กรอบกล่องมนสี่เหลี่ยม (อ้างอิง 3..29 ในกริด 32)
    let l = scale(3, size as u32) as f32;
    let t = l;
    let r = scale(29, size as u32) as f32;
    let b = r;
    let rad = scale(7, size as u32) as f32;

    // ปุ่มคีย์บอร์ด 3x3 (อ้างอิง 7/13/19 ขนาด 5 ในกริด 32)
    let key_w = scale(5, size as u32);
    let key_rad = scale(1, size as u32) as f32;
    let keys: Vec<(i32, i32)> = [7, 13, 19]
        .iter()
        .flat_map(|&kx| {
            [7, 13, 19]
                .iter()
                .map(move |&ky| (scale(kx, size as u32), scale(ky, size as u32)))
        })
        .collect();

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            let idx = ((y * size + x) * 4) as usize;

            if !in_round_rect(fx, fy, l, t, r, b, rad) {
                continue;
            }

            let on_key = keys.iter().any(|&(kx, ky)| {
                in_round_rect(
                    fx,
                    fy,
                    kx as f32,
                    ky as f32,
                    (kx + key_w) as f32,
                    (ky + key_w) as f32,
                    key_rad,
                )
            });

            let (r8, g8, b8) = if on_key { WHITE } else { BRAND };
            px[idx] = r8;
            px[idx + 1] = g8;
            px[idx + 2] = b8;
            px[idx + 3] = 255;
        }
    }
    px
}

/// เข้ารหัสภาพ RGBA เป็น PNG (ใช้เป็นข้อมูลภาพในไฟล์ .ico)
fn encode_png(rgba: &[u8], size: u32) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    PngEncoder::new(&mut out).write_image(rgba, size, size, ExtendedColorType::Rgba8)?;
    Ok(out)
}

/// ประกอบไฟล์ .ico จากภาพ PNG หลายขนาด
///
/// โครงสร้างไฟล์: ICONDIR (6 ไบต์) + ICONDIRENTRY (16 ไบต์ ต่อภาพ) + ข้อมูลภาพ
fn ico_container(frames: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();

    // ICONDIR
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // 1 = ไอคอน (2 = เคอร์เซอร์)
    out.extend_from_slice(&(frames.len() as u16).to_le_bytes());

    // ICONDIRENTRY
    let mut offset = 6u32 + 16 * frames.len() as u32;
    for (size, data) in frames {
        // ขนาด 256 เก็บเป็น 0 ตามสเปก
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        out.push(dim);
        out.push(dim);
        out.push(0); // จำนวนสีใน palette (0 = ไม่มี)
        out.push(0); // reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // color planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }

    // ข้อมูลภาพ
    for (_, data) in frames {
        out.extend_from_slice(data);
    }

    out
}

/// สร้างไฟล์ .ico ครบทุกขนาด (ใช้ทำไอคอนของ installer และของตัวโปรแกรม)
pub fn build_ico_bytes() -> Result<Vec<u8>> {
    let mut frames = Vec::with_capacity(ICO_SIZES.len());
    for size in ICO_SIZES {
        let rgba = build_rgba_at(size);
        frames.push((size, encode_png(&rgba, size)?));
    }
    Ok(ico_container(&frames))
}

/// เขียนไฟล์ .ico ลงดิสก์
pub fn export_ico(path: &Path) -> Result<()> {
    let bytes = build_ico_bytes()?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, &bytes)?;
    if !path.exists() {
        bail!("เขียนไฟล์ {} ไม่สำเร็จ", path.display());
    }
    Ok(())
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
        for (p, src) in dst
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(rgba.as_chunks::<4>().0.iter())
        {
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
        let stride = (SIZE as usize).div_ceil(32) * 4;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// ตำแหน่งปุ่มคีย์บอร์ด 3x3 บนไอคอน 32x32 (มุมบนซ้ายของแต่ละปุ่ม)
    const LEGACY_KEYS: [(i32, i32); 9] = [
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

    /// วิธีวาดเดิม (ตรึงไว้เป็นเบสไลน์ เพื่อกันไม่ให้ไอคอนใน tray เปลี่ยนไป)
    fn legacy_build_rgba() -> Vec<u8> {
        let w = 32usize;
        let mut px = vec![0u8; w * w * 4];
        for y in 0..32i32 {
            for x in 0..32i32 {
                let fx = x as f32;
                let fy = y as f32;
                let idx = (y as usize * w + x as usize) * 4;

                if !in_round_rect(fx, fy, 3.0, 3.0, 29.0, 29.0, 7.0) {
                    continue;
                }

                let on_key = LEGACY_KEYS.iter().any(|&(kx, ky)| {
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

    /// อ่านรายการขนาดภาพออกจากไฟล์ .ico (0 แปลว่า 256)
    fn ico_sizes(bytes: &[u8]) -> Vec<u8> {
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        (0..count)
            .map(|i| {
                let off = 6 + i * 16;
                bytes[off]
            })
            .collect()
    }

    /// ตำแหน่งและความยาวของข้อมูลภาพแต่ละรายการ
    fn ico_entries(bytes: &[u8]) -> Vec<(u32, u32)> {
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        (0..count)
            .map(|i| {
                let off = 6 + i * 16;
                let size = u32::from_le_bytes([bytes[off + 8], bytes[off + 9], bytes[off + 10], bytes[off + 11]]);
                let offset = u32::from_le_bytes([
                    bytes[off + 12],
                    bytes[off + 13],
                    bytes[off + 14],
                    bytes[off + 15],
                ]);
                (offset, size)
            })
            .collect()
    }

    #[test]
    fn build_rgba_at_32_is_identical_to_legacy() {
        // ไอคอนใน system tray ต้องไม่เปลี่ยนไปจากเดิม
        assert_eq!(build_rgba_at(32), legacy_build_rgba());
    }

    #[test]
    fn build_rgba_at_keeps_shape_and_colors() {
        let px = build_rgba_at(16);
        assert_eq!(px.len(), 16 * 16 * 4);

        // มุมซ้ายบนสุดโปร่งใส (นอกกรอบกล่อง)
        assert_eq!(px[3], 0, "มุมภาพต้องโปร่งใส");

        // ต้องมีทั้งสีกล่อง (แบรนด์) และสีปุ่ม (ขาว) อยู่ในภาพ
        let brand_pixels = px
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] == BRAND.0 && p[1] == BRAND.1 && p[2] == BRAND.2)
            .count();
        let white_pixels = px
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] == WHITE.0 && p[1] == WHITE.1 && p[2] == WHITE.2)
            .count();
        assert!(brand_pixels > 0, "ไม่พบสีกล่องแบรนด์ในไอคอน");
        assert!(white_pixels > 0, "ไม่พบปุ่มสีขาวในไอคอน");

        // ปุ่มต้องเล็กกว่ากล่องเสมอ (คือไอคอนไม่กลายเป็นก้อนสีขาวทั้งหมด)
        assert!(
            white_pixels < brand_pixels,
            "ปุ่มสีขาวต้องน้อยกว่าพื้นกล่อง ({white_pixels} vs {brand_pixels})"
        );
    }

    #[test]
    fn ico_container_header_is_valid() {
        let frames = vec![(16u32, vec![1u8, 2, 3]), (32u32, vec![4u8, 5])];
        let bytes = ico_container(&frames);

        assert_eq!(&bytes[0..2], &0u16.to_le_bytes(), "reserved ต้องเป็น 0");
        assert_eq!(&bytes[2..4], &1u16.to_le_bytes(), "type ต้องเป็น 1 (ไอคอน)");
        assert_eq!(&bytes[4..6], &2u16.to_le_bytes(), "จำนวนภาพต้องตรงกับเฟรม");
        assert_eq!(ico_sizes(&bytes), vec![16, 32]);
    }

    #[test]
    fn ico_container_entries_point_to_real_data() {
        let frames = vec![(16u32, vec![7u8; 10]), (32u32, vec![8u8; 20])];
        let bytes = ico_container(&frames);
        let entries = ico_entries(&bytes);

        // offset ต้องเริ่มหลัง header + dir entries
        assert_eq!(entries[0].0, 6 + 16 * 2);
        // offset ถัดไปต่อท้ายข้อมูลภาพก่อนหน้า
        assert_eq!(entries[1].0, entries[0].0 + entries[0].1);
        // ข้อมูลต้องไม่ล้นไฟล์
        for (offset, size) in entries {
            assert!((offset + size) as usize <= bytes.len());
        }
    }

    #[test]
    fn build_ico_bytes_contains_every_size() {
        let bytes = build_ico_bytes().expect("สร้าง .ico ไม่สำเร็จ");

        assert_eq!(ico_sizes(&bytes), vec![16, 24, 32, 48, 64, 128, 0]);
        // ขนาด 256 เก็บเป็น 0 ตามสเปก ICO
        assert_eq!(ico_sizes(&bytes).last().copied(), Some(0));

        for (offset, size) in ico_entries(&bytes) {
            assert!(size > 0);
            assert!((offset + size) as usize <= bytes.len());
        }
    }

    #[test]
    fn committed_icon_asset_is_up_to_date() {
        // ไฟล์ assets/mackeys.ico ต้องตรงกับที่โค้ดวาดอยู่เสมอ
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("mackeys.ico");
        assert!(path.exists(), "ไม่พบไฟล์ {}", path.display());

        let committed = fs::read(&path).expect("อ่าน assets/mackeys.ico ไม่สำเร็จ");
        let generated = build_ico_bytes().expect("สร้าง .ico ไม่สำเร็จ");
        assert_eq!(
            committed.len(),
            generated.len(),
            "assets/mackeys.ico ไม่ตรงกับโค้ด — ให้รัน `mackeys icon-export assets\\mackeys.ico` แล้ว commit ใหม่"
        );
        assert_eq!(committed, generated, "assets/mackeys.ico ไม่ตรงกับโค้ด");
    }
}
