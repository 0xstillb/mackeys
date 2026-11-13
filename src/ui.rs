//! ระบบดีไซน์ของหน้าต่างตั้งค่า — โทนมืดสมัย (Fluent-like)
//!
//! รวมทุกชิ้นส่วนที่ใช้ซ้ำไว้ที่นี่: โทนสี, การ์ด, สวิตช์แบบ iOS/Windows 11,
//! คีย์แคปสไตล์ macOS, ป้ายสถานะ, ปุ่ม และโลโก้

use egui::{
    Align, Color32, CornerRadius, Margin, Rect, Response, RichText, Sense, Shape, Stroke, Ui, Vec2,
};

/// ระยะห่างมาตรฐาน (spacing scale) — ใช้ทุกที่แทนตัวเลขมั่ว ๆ
///
/// ขนาดเป็น 4 / 8 / 12 / 16 / 24 / 32 เท่านั้น เพื่อให้ทุกหน้า "หายใจ" เหมือนกัน
pub mod space {
    /// 4 px — ระยะห่างจิ๋ว (ในกลุ่มข้อความเดียวกัน)
    pub const XS: f32 = 4.0;
    /// 8 px — ระยะห่างเล็ก (ระหว่างแถวในกลุ่ม)
    pub const SM: f32 = 8.0;
    /// 12 px — ระยะห่างกลาง (ก่อน/หลังกลุ่มย่อย)
    pub const MD: f32 = 12.0;
    /// 16 px — ระยะห่างใหญ่ (ระหว่างการ์ด, ขอบจอ)
    pub const LG: f32 = 16.0;
    /// 24 px — ระยะห่างหลัก (หัวข้อ → เนื้อหา)
    pub const XL: f32 = 24.0;
    /// 32 px — ระยะห่างใหญ่พิเศษ
    pub const XXL: f32 = 32.0;
}

/// ค่าคงที่ด้านเลย์เอาต์ของหน้าต่าง (ใช้ร่วมกันระหว่างการวาดกับเทสต์)
///
/// เก็บไว้เป็นตัวเลขล้วน ๆ เพื่อให้เทสต์ตรวจได้ว่า
/// - รายการเมนูไม่ชิดเส้นแบ่งของแถบด้านข้าง
/// - หัวข้อหน้ากับขอบซ้ายของการ์ดตรงกันเป๊ะ
/// - ทุกระยะห่างเป็นพหุลของ 4 (ตาม spacing scale)
pub mod layout {
    /// ความกว้างแถบเมนูด้านข้าง
    pub const NAV_WIDTH: f32 = 224.0;
    /// ระยะห่างภายในแถบเมนู (ซ้าย/ขวา) — กันรายการที่เลือกชิดเส้นแบ่ง
    pub const NAV_PAD_X: f32 = 12.0;
    /// ระยะห่างภายในแถบเมนู (บน/ล่าง)
    pub const NAV_PAD_Y: f32 = 16.0;
    /// ความสูงของรายการเมนู (ให้ทุกรายการสูงเท่ากัน)
    pub const NAV_ITEM_HEIGHT: f32 = 40.0;
    /// ความสูงแถบบน
    pub const TOP_BAR_HEIGHT: f32 = 64.0;
    /// ระยะห่างขอบพื้นที่เนื้อหา (ซ้าย/ขวา) — กันหัวข้อหน้าไปชิดแถบเมนู
    pub const CONTENT_PAD_X: f32 = 24.0;
    /// ระยะห่างขอบพื้นที่เนื้อหา (บน/ล่าง)
    pub const CONTENT_PAD_Y: f32 = 24.0;
    /// ระยะห่างแนวนอนของการ์ด — ต้องเป็น 0
    /// เพื่อให้ขอบซ้ายของการ์ดตรงกับหัวข้อหน้าพอดี (ไม่เหลื่อม ไม่เหลือ)
    pub const CARD_GAP_X: f32 = 0.0;
    /// ระยะห่างแนวตั้งระหว่างการ์ด
    pub const CARD_GAP_Y: f32 = 4.0;

    /// ขอบซ้ายของพื้นที่เนื้อหา (หัวข้อหน้าเริ่มตรงนี้)
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn content_left_edge(nav_width: f32) -> f32 {
        nav_width + CONTENT_PAD_X
    }

    /// ขอบซ้ายของการ์ด (ต้องตรงกับ `content_left_edge`)
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn card_left_edge(nav_width: f32) -> f32 {
        content_left_edge(nav_width) + CARD_GAP_X
    }

    /// ความกว้างพื้นที่เนื้อหาที่วางได้จริง (หลังหักแถบเมนู + ระยะห่างสองข้าง)
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn content_width(window_width: f32) -> f32 {
        (window_width - NAV_WIDTH - 2.0 * CONTENT_PAD_X).max(0.0)
    }

    /// ความกว้างรายการเมนูที่วาดได้ (เหลือระยะห่างเท่ากันทั้งสองข้าง)
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn nav_item_width(panel_width: f32) -> f32 {
        (panel_width - 2.0 * NAV_PAD_X).max(0.0)
    }
}

/// ขนาดตัวอักษรมาตรฐาน (type scale) — ใช้ทุกที่
pub mod font {
    /// 11 px — คำอธิบายเล็กที่สุด (ล่างสุดของการ์ด, เวอร์ชัน)
    pub const TINY: f32 = 11.0;
    /// 12 px — คำอธิบาย/ป้ายเล็ก
    pub const SMALL: f32 = 12.0;
    /// 13 px — ข้อความปกติในกลุ่มตัวเลือก
    pub const BODY: f32 = 13.0;
    /// 13.5 px — ปุ่ม/รายการเมนู
    pub const BODY_LG: f32 = 13.5;
    /// 15 px — หัวข้อการ์ด
    pub const TITLE: f32 = 15.0;
    /// 17 px — ชื่อแอปในแถบบน
    pub const TITLE_LG: f32 = 17.0;
    /// 20 px — ชื่อใหญ่ในการ์ดต้อนรับ
    pub const DISPLAY: f32 = 20.0;
    /// 22 px — หัวข้อหน้า
    pub const HEADING: f32 = 22.0;
    /// 26 px — ตัวเลขสถิติ
    pub const STAT: f32 = 26.0;
}

/// ความโค้งมุมมาตรฐาน (corner radius)
pub mod radius {
    /// 6 px — คีย์แคป
    pub const SM: f32 = 6.0;
    /// 8 px — ช่องกรอก/ปุ่มเล็ก/วิดเจ็ต
    pub const MD: f32 = 8.0;
    /// 9 px — ปุ่ม
    pub const LG: f32 = 9.0;
    /// 10 px — ป้าย/เมนู
    pub const PILL: f32 = 10.0;
    /// 14 px — การ์ด
    pub const XL: f32 = 14.0;
}

/// ความกว้างมาตรฐานของช่องเลือก (dropdown) — ให้ทุกช่องเรียงแนวกัน
pub const COMBO_WIDTH: f32 = 200.0;

/// ความกว้างคอลัมน์คีย์แคปในหน้าคีย์ลัด (ให้ dropdown ทุกแถวตรงกัน)
pub const KEYCAP_COL_WIDTH: f32 = 236.0;

/// ความกว้างคอลัมน์ "ปุ่ม" ในบรรทัดสถานะ (ให้ทุกแถวตรงกัน)
pub const STATUS_KEYS_COL_WIDTH: f32 = 88.0;

/// ความกว้างคอลัมน์ "คำอธิบาย action" ในบรรทัดสถานะ (ให้ทุกแถวตรงกัน)
pub const STATUS_ACTION_COL_WIDTH: f32 = 124.0;

/// ความสูงของบรรทัดสถานะ (พอสำหรับตัวอักษร 12 px)
pub const STATUS_ROW_HEIGHT: f32 = 20.0;

/// จัดคอลัมน์กว้างคงที่ (ให้ข้อความในแต่ละแถวตรงกัน)
///
/// **อย่าใช้ `allocate_ui_with_layout` ตรง ๆ** — มันจัดพื้นที่เท่าที่ "ใช้จริง" เท่านั้น
/// (เอกสาร egui: *"the amount of space actually used (min_rect) will be allocated"*)
/// → คอลัมน์จะหดตามความยาวข้อความ → แต่ละแถวข้ามกัน ไม่ตรงกัน
///
/// ต้องใช้ `allocate_exact_size` ที่จองพื้นที่ตายตัวตามที่ขอ
pub fn fixed_col<R>(ui: &mut Ui, width: f32, height: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    add(&mut child)
}

/// โทนสีทั้งหมดของแอป
pub struct Palette;

impl Palette {
    /// พื้นหลังหลัก (สีมืดลึก)
    pub const BG: Color32 = Color32::from_rgb(13, 15, 20);
    /// พื้นหลังแถบเมนูด้านข้าง
    pub const NAV: Color32 = Color32::from_rgb(17, 20, 26);
    /// พื้นการ์ด
    pub const CARD: Color32 = Color32::from_rgb(22, 25, 33);
    /// พื้นการ์ด/ช่องกรอกที่ "ยก" ขึ้น
    pub const CARD_HI: Color32 = Color32::from_rgb(29, 33, 43);
    /// พื้นช่องกรอกข้อความ
    pub const FIELD: Color32 = Color32::from_rgb(15, 17, 23);
    /// เส้นขอบ
    pub const BORDER: Color32 = Color32::from_rgb(37, 42, 54);
    /// เส้นขอบตอน hover
    pub const BORDER_HI: Color32 = Color32::from_rgb(58, 65, 84);
    /// ข้อความหลัก
    pub const TEXT: Color32 = Color32::from_rgb(233, 235, 241);
    /// ข้อความรอง
    pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 158, 175);
    /// ข้อความจาง (คำอธิบายยาว)
    pub const TEXT_FAINT: Color32 = Color32::from_rgb(104, 112, 130);
    /// สีหลัก (periwinkle)
    pub const ACCENT: Color32 = Color32::from_rgb(108, 140, 255);
    pub const ACCENT_HI: Color32 = Color32::from_rgb(136, 163, 255);
    /// เงาเรืองแสงรอบช่องกรอกตอนโฟกัส (ชั้นใกล้)
    pub const ACCENT_GLOW: Color32 = Color32::from_rgba_unmultiplied_const(108, 140, 255, 90);
    /// เงาเรืองแสงรอบช่องกรอกตอนโฟกัส (ชั้นไกล โปร่งกว่า)
    pub const ACCENT_GLOW_FAINT: Color32 = Color32::from_rgba_unmultiplied_const(108, 140, 255, 38);
    /// พื้นของสีหลักแบบจาง (สำหรับพื้นหลัง + เส้นขอบ)
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(36, 46, 82);
    pub const OK: Color32 = Color32::from_rgb(74, 222, 128);
    pub const OK_DIM: Color32 = Color32::from_rgb(22, 56, 38);
    pub const WARN: Color32 = Color32::from_rgb(251, 191, 36);
    pub const WARN_DIM: Color32 = Color32::from_rgb(62, 49, 16);
    pub const DANGER: Color32 = Color32::from_rgb(248, 113, 113);
    pub const DANGER_DIM: Color32 = Color32::from_rgb(68, 30, 34);
}

/// ตั้งค่าสไตล์ทั้งระบบ (เรียกครั้งเดียวตอนเปิดโปรแกรม)
pub fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();

    style.visuals.dark_mode = true;
    style.visuals.panel_fill = Palette::BG;
    style.visuals.window_fill = Palette::CARD;
    style.visuals.extreme_bg_color = Palette::FIELD;
    style.visuals.faint_bg_color = Palette::CARD;
    style.visuals.code_bg_color = Palette::FIELD;
    style.visuals.hyperlink_color = Palette::ACCENT;
    style.visuals.warn_fg_color = Palette::WARN;
    style.visuals.error_fg_color = Palette::DANGER;
    style.visuals.weak_text_color = Some(Palette::TEXT_DIM);
    style.visuals.window_corner_radius = CornerRadius::same(14);
    style.visuals.menu_corner_radius = CornerRadius::same(10);
    style.visuals.window_stroke = Stroke::new(1.0, Palette::BORDER);
    style.visuals.selection.bg_fill = Palette::ACCENT_DIM;
    style.visuals.selection.stroke = Stroke::new(1.0, Palette::ACCENT);

    let w = &mut style.visuals.widgets;
    w.noninteractive.bg_fill = Palette::CARD;
    w.noninteractive.weak_bg_fill = Palette::CARD;
    w.noninteractive.bg_stroke = Stroke::new(1.0, Palette::BORDER);
    w.noninteractive.corner_radius = CornerRadius::same(8);

    w.inactive.bg_fill = Palette::CARD_HI;
    w.inactive.weak_bg_fill = Palette::CARD_HI;
    w.inactive.bg_stroke = Stroke::new(1.0, Palette::BORDER);
    w.inactive.corner_radius = CornerRadius::same(8);

    w.hovered.bg_fill = Palette::ACCENT_DIM;
    w.hovered.weak_bg_fill = Palette::ACCENT_DIM;
    w.hovered.bg_stroke = Stroke::new(1.0, Palette::ACCENT);
    w.hovered.corner_radius = CornerRadius::same(8);

    w.active.bg_fill = Palette::ACCENT;
    w.active.weak_bg_fill = Palette::ACCENT;
    w.active.bg_stroke = Stroke::new(1.0, Palette::ACCENT_HI);
    w.active.fg_stroke = Stroke::new(1.0, Palette::BG);
    w.active.corner_radius = CornerRadius::same(8);

    w.open.bg_fill = Palette::CARD_HI;
    w.open.weak_bg_fill = Palette::CARD_HI;
    w.open.bg_stroke = Stroke::new(1.0, Palette::BORDER_HI);
    w.open.corner_radius = CornerRadius::same(8);

    style.spacing.item_spacing = Vec2::new(space::SM, space::SM);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.slider_width = 140.0;

    // ขนาดตัวอักษรให้อ่านสบายและดูเป็นสมัย
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(font::HEADING, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(font::BODY, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(font::BODY_LG, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(font::SMALL, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
    );

    ctx.set_style_of(egui::Theme::Dark, style);
}

/// การ์ดพื้นฐาน — กรอบมน มีเส้นขอบบาง
///
/// การ์ดจะยืดเต็มความกว้างที่มี เพื่อให้ทุกหน้าเรียงเป็นคอลัมน์เดียวกัน
/// (ถ้าไม่ยืด การ์ดที่เนื้อหาน้อยจะกลายเป็นแถบแคบ ๆ หน้าตาไม่เป็นเรียบ)
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(Palette::CARD)
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .corner_radius(CornerRadius::same(radius::XL as u8))
        .inner_margin(Margin::same(space::LG as i8))
        .outer_margin(Margin::symmetric(
            layout::CARD_GAP_X as i8,
            layout::CARD_GAP_Y as i8,
        ))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// การ์ดที่มีหัวข้อ + คำอธิบาย
pub fn titled_card(
    ui: &mut Ui,
    title: &str,
    subtitle: Option<&str>,
    add: impl FnOnce(&mut Ui),
) {
    card(ui, |ui| {
        ui.label(RichText::new(title).size(font::TITLE).strong().color(Palette::TEXT));
        if let Some(s) = subtitle {
            ui.label(RichText::new(s).size(font::SMALL).color(Palette::TEXT_FAINT));
            ui.add_space(space::SM);
        }
        add(ui);
    });
}

/// สวิตช์แบบ iOS / Windows 11 (มีแอนิเมชัน)
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let size = Vec2::new(42.0, 23.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let anim = ui.ctx().animate_bool(response.id, *on);
        let track = Rect::from_center_size(rect.center(), size);
        let track_color = lerp_color(Palette::CARD_HI, Palette::ACCENT, anim);
        ui.painter().rect_filled(track, size.y / 2.0, track_color);
        ui.painter().rect_stroke(
            track,
            size.y / 2.0,
            Stroke::new(1.0, lerp_color(Palette::BORDER_HI, Palette::ACCENT_HI, anim)),
            egui::StrokeKind::Inside,
        );

        let x = egui::emath::lerp(rect.left() + 11.5..=rect.right() - 11.5, anim);
        let knob = Rect::from_center_size(
            egui::pos2(x, rect.center().y),
            Vec2::splat(17.0),
        );
        ui.painter().rect_filled(knob, 8.5, Color32::WHITE);
    }

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response
}

/// ป้ายสถานะ (pill)
pub fn pill(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(radius::PILL as u8))
        .inner_margin(Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(font::SMALL).strong().color(fg));
        });
}

/// กรอบคีย์แยกชิ้น (สไตล์ macOS)
pub fn keycap(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .fill(Palette::CARD_HI)
        .stroke(Stroke::new(1.0, Palette::BORDER_HI))
        .corner_radius(CornerRadius::same(radius::SM as u8))
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text.to_uppercase())
                    .size(font::SMALL)
                    .strong()
                    .color(Palette::TEXT),
            );
        });
}

/// แสดงคอมโบคีย์ลัด เช่น "ctrl+shift+3" เป็นคีย์แยกชิ้น
pub fn keycap_combo(ui: &mut Ui, combo: &str) {
    let parts: Vec<String> = combo
        .split('+')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // ใช้ `horizontal` ไม่ใช่ `horizontal_wrapped`
    // เพราะเมื่อซ้อนอยู่ใน `horizontal_wrapped` (แถวตัวอย่างในการ์ด)
    // egui จะวัดความกว้างของ layout ที่ซ้อนกันได้ 0 → ชิ้นสุดท้ายล้นออกนอกการ์ด
    ui.horizontal(|ui| {
        for (i, p) in parts.iter().enumerate() {
            keycap(ui, p);
            if i + 1 < parts.len() {
                ui.label(RichText::new("+").size(font::SMALL).color(Palette::TEXT_FAINT));
            }
        }
    });
}

/// คีย์แคปคู่กับคำอธิบาย ในกล่องเดียวกัน
///
/// **ห้ามเรียกจากใน `horizontal_wrapped`** — egui วัดความกว้างของ layout ที่ซ้อนกัน
/// (ซึ่งกินความกว้างทั้งหมดของบรรทัด) ไม่ถูกต้อง → ชิ้นสุดท้ายล้นออกนอกการ์ด
/// ให้วางในคอลัมน์ที่กำหนดความกว้างชัดเจนแทน
pub fn keycap_with_label(ui: &mut Ui, combo: &str, desc: &str) {
    ui.horizontal(|ui| {
        keycap_combo(ui, combo);
        ui.add_space(space::SM);
        ui.label(
            RichText::new(desc)
                .size(font::SMALL)
                .color(Palette::TEXT_FAINT),
        );
    });
}

/// คีย์ลัดสำหรับ **แสดงผลให้ผู้ใช้** (ฟังก์ชันบริสุทธิ์ — เทสต์ได้)
///
/// ในโหมด Mac ปุ่ม `Alt` ฝั่งซ้ายทำหน้าที่เป็น `Ctrl` → ผู้ใช้กด `Alt` ไม่ใช่ `Ctrl`
/// ถ้าแสดง `CTRL` ต่อไป ผู้ใช้จะกดผิดปุ่ม
///
/// *ค่าในไฟล์ config ยังเป็น `ctrl+...` เสมอ* เพราะการลงทะเบียน hotkey ต้องใช้ `Ctrl`
/// (ถ้าใช้ `Alt` จะไม่มีวันยิง เพราะโหมด Mac กลืนปุ่ม Alt ทิ้ง)
pub fn display_keys(keys: &str, mac_mode: bool) -> String {
    if !mac_mode {
        return keys.to_string();
    }
    keys.split('+')
        .map(|p| {
            let t = p.trim();
            if t.eq_ignore_ascii_case("ctrl") {
                "alt".to_string()
            } else {
                t.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// ปุ่มหลัก (สีเน้น)
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    primary_button_w(ui, text, 0.0)
}

/// ปุ่มหลักกำหนดความกว้าง (ให้ปุ่มในชุดเดียวกันเรียงกันเป็นตาราง)
pub fn primary_button_w(ui: &mut Ui, text: &str, width: f32) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(font::BODY_LG).strong().color(Color32::WHITE))
            .fill(Palette::ACCENT)
            .stroke(Stroke::new(1.0, Palette::ACCENT_HI))
            .corner_radius(CornerRadius::same(radius::LG as u8))
            .min_size(Vec2::new(width, 32.0)),
    )
}

/// ปุ่มรอง (โปร่งใส + เส้นขอบ)
pub fn ghost_button(ui: &mut Ui, text: &str) -> Response {
    ghost_button_w(ui, text, 0.0)
}

/// ปุ่มรองกำหนดความกว้าง (ให้ปุ่มในชุดเดียวกันเรียงกันเป็นตาราง)
pub fn ghost_button_w(ui: &mut Ui, text: &str, width: f32) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(font::BODY).color(Palette::TEXT_DIM))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, Palette::BORDER))
            .corner_radius(CornerRadius::same(radius::LG as u8))
            .min_size(Vec2::new(width, 32.0)),
    )
}

/// ปุ่มเล็กสำหรับไอคอน (เช่น ✕ หรือ ↑)
pub fn icon_button(ui: &mut Ui, text: &str, hover: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(font::BODY).color(Palette::TEXT_DIM))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, Palette::BORDER))
            .corner_radius(CornerRadius::same(radius::MD as u8))
            .min_size(Vec2::new(30.0, 28.0))
            .sense(Sense::click()),
    )
    .on_hover_text(hover)
}

/// ระยะระหว่างจุดสามจุดในโลโก้ (คืนค่าเป็นระยะจากศูนย์กลาง) — ฟังก์ชันบริสุทธิ์ เทสต์ได้
///
/// ต้องมากกว่าเส้นผ่านศูนย์กลางของจุด ไม่งั้นจุดจะซ้อนกันกลายเป็น "ขีด" เส้นเดียว
pub fn logo_dot_offsets(size: f32) -> [f32; 3] {
    let gap = size * 0.21;
    [-gap, 0.0, gap]
}

/// รัศมีจุดในโลโก้
pub fn logo_dot_radius(size: f32) -> f32 {
    size * 0.115 / 2.0
}

/// โลโก้: สี่เหลี่ยมมนสีหลัก พร้อมจุดสามจุดคล้ายไอคอนใน tray
pub fn logo(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    let radius = size * 0.28;

    p.rect_filled(rect, radius, Palette::ACCENT);
    // ไล่เฉดด้านบนให้ดูมีมิติ
    let top = Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.center().y));
    p.rect_filled(top, radius, Palette::ACCENT_HI);

    // จุดสามจุด (คีย์บอร์ด)
    let r = logo_dot_radius(size);
    for off in logo_dot_offsets(size) {
        let c = egui::pos2(rect.center().x + off, rect.center().y);
        p.rect_filled(Rect::from_center_size(c, Vec2::splat(r * 2.0)), r, Color32::WHITE);
    }
}

/// ไอคอนเวกเตอร์ที่วาดเองด้วย painter
///
/// วาดเองแทนการใช้ตัวอักษรสัญลักษณ์ เพราะฟอนต์ที่มีใน Windows
/// ไม่ครอบคลุมทุกตัว (บางตัวออกมาเป็นกล่องสี่เหลี่ยม "tofu")
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// แผงภาพรวม — ตาราง 4 ช่อง
    Dashboard,
    /// คีย์บอร์ด
    Keyboard,
    /// โลก (ภาษา)
    Globe,
    /// สไลเดอร์ (ตัวเลือกระบบ)
    Sliders,
}

impl Icon {
    /// วาดไอคอนใหญ่เท่ากับสี่เหลี่ยมจัตุรัส `rect`
    pub fn draw(&self, p: &egui::Painter, rect: Rect, color: Color32) {
        let s = rect.width().min(rect.height());
        let stroke = Stroke::new((s * 0.085).max(1.2), color);

        match self {
            Icon::Dashboard => {
                // ช่องบนซ้ายเต็ม อีกสามช่องเป็นเส้น (ลุ้นแบบ dashboard)
                let gap = s * 0.18;
                let cell = (s - gap) / 2.0;
                let cells = [
                    (0.0, 0.0, true),
                    (1.0, 0.0, false),
                    (0.0, 1.0, false),
                    (1.0, 1.0, false),
                ];
                for (dx, dy, filled) in cells {
                    let r = Rect::from_min_max(
                        egui::pos2(
                            rect.left() + dx * (cell + gap),
                            rect.top() + dy * (cell + gap),
                        ),
                        egui::pos2(
                            rect.left() + dx * (cell + gap) + cell,
                            rect.top() + dy * (cell + gap) + cell,
                        ),
                    );
                    if filled {
                        p.rect_filled(r, cell * 0.3, color);
                    } else {
                        p.rect_stroke(r, cell * 0.3, stroke, egui::StrokeKind::Inside);
                    }
                }
            }
            Icon::Keyboard => {
                // กรอบคีย์บอร์ด + แถวปุ่มเล็ก
                let r = Rect::from_min_max(
                    egui::pos2(rect.left(), rect.top() + s * 0.1),
                    egui::pos2(rect.right(), rect.bottom() - s * 0.1),
                );
                p.rect_stroke(r, s * 0.18, stroke, egui::StrokeKind::Inside);

                let dot = s * 0.075;
                let y1 = r.top() + r.height() * 0.34;
                let y2 = r.top() + r.height() * 0.63;
                let xs = [0.22, 0.42, 0.62, 0.82];
                for x in xs {
                    let c = egui::pos2(r.left() + r.width() * x, y1);
                    p.circle_filled(c, dot, color);
                }
                // แถบพิมพ์เว้น (spacebar)
                let bar = Rect::from_center_size(
                    egui::pos2(r.center().x, y2),
                    Vec2::new(r.width() * 0.44, s * 0.075),
                );
                p.rect_filled(bar, s * 0.04, color);
            }
            Icon::Globe => {
                // วงกลม + เส้นละติจูด + เส้นศูนย์สูตร
                let c = rect.center();
                let r = s * 0.46;
                p.circle_stroke(c, r, stroke);
                // เส้นแนวนอน (แนวเส้นศูนย์สูตร)
                p.line_segment(
                    [egui::pos2(c.x - r, c.y), egui::pos2(c.x + r, c.y)],
                    stroke,
                );
                // วงรีแนวตั้ง (เส้นละติจูด)
                let mut pts = Vec::with_capacity(25);
                for i in 0..25 {
                    let t = i as f32 / 24.0 * std::f32::consts::TAU;
                    pts.push(egui::pos2(
                        c.x + r * 0.44 * t.cos(),
                        c.y + r * t.sin(),
                    ));
                }
                p.add(Shape::closed_line(pts, stroke));
            }
            Icon::Sliders => {
                // สามแถบปรับค่า แต่ละแถบมีปุ่มเลื่อนคนละตำแหน่ง
                let rows = [
                    (0.0, 0.55),
                    (1.0, 0.30),
                    (2.0, 0.70),
                ];
                for (i, knob_x) in rows {
                    let y = rect.top() + s * (0.22 + i * 0.28);
                    p.line_segment(
                        [egui::pos2(rect.left() + s * 0.06, y), egui::pos2(rect.right() - s * 0.06, y)],
                        stroke,
                    );
                    let kx = rect.left() + s * (0.06 + knob_x * 0.88);
                    p.circle_filled(egui::pos2(kx, y), s * 0.115, color);
                    // ขอบวงกลมสีพื้นหลัง เพื่อให้ดูเป็นปุ่มเลื่อน
                    p.circle_stroke(egui::pos2(kx, y), s * 0.115, Stroke::new(1.0, Palette::CARD));
                }
            }
        }
    }
}

/// รายการเมนูด้านข้าง (พร้อมแถบไฮไลต์แบบแอนิเมชัน)
pub fn nav_item(ui: &mut Ui, id: &str, icon: Icon, label: &str, selected: bool) -> Response {
    /// ความสูงรายการเมนู (ให้ทุกรายการสูงเท่ากัน)
    const NAV_ITEM_H: f32 = layout::NAV_ITEM_HEIGHT;
    /// ระยะจากขอบซ้ายถึงกลางไอคอน
    const ICON_CX: f32 = 22.0;
    /// ระยะจากขอบซ้ายถึงข้อความ
    const TEXT_X: f32 = 42.0;

    let size = Vec2::new(ui.available_width(), NAV_ITEM_H);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let anim = ui.ctx().animate_bool(response.id, selected);
        if anim > 0.01 {
            ui.painter().rect_filled(
                rect,
                radius::LG,
                lerp_color(Palette::CARD, Palette::ACCENT_DIM, anim),
            );
        }
        if selected {
            // แถบเน้นด้านซ้าย
            let bar = Rect::from_min_max(
                egui::pos2(rect.left() + 1.0, rect.top() + 10.0),
                egui::pos2(rect.left() + 3.5, rect.bottom() - 10.0),
            );
            ui.painter().rect_filled(bar, 1.5, Palette::ACCENT);
        }

        let text_color = if selected {
            Palette::TEXT
        } else if response.hovered() {
            Palette::TEXT
        } else {
            Palette::TEXT_DIM
        };

        // ไอคอนเวกเตอร์
        let icon_rect = Rect::from_center_size(
            egui::pos2(rect.left() + ICON_CX, rect.center().y),
            Vec2::splat(17.0),
        );
        icon.draw(ui.painter(), icon_rect, text_color);

        ui.painter().text(
            egui::pos2(rect.left() + TEXT_X, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::new(font::BODY_LG, egui::FontFamily::Proportional),
            text_color,
        );
    }

    let _ = id;
    response
}

/// ผสมสีสองสี (t = 0 → a, t = 1 → b)
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        a.r() + ((b.r() as f32 - a.r() as f32) * t) as u8,
        a.g() + ((b.g() as f32 - a.g() as f32) * t) as u8,
        a.b() + ((b.b() as f32 - a.b() as f32) * t) as u8,
        a.a() + ((b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

/// ช่องข้อความแบบมีกรอบกลม (ใช้แทน TextEdit ปกติ)
pub fn field(ui: &mut Ui, text: &mut String, width: f32) -> Response {
    let id = ui.scope_id().with("mackeys_field_plain");
    let focused = ui.memory(|m| m.has_focus(id));

    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, FIELD_HEIGHT), Sense::hover());
    paint_field(ui, rect, focused);

    let inner = rect.shrink2(Vec2::new(10.0, 8.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.add(
        egui::TextEdit::singleline(text)
            .id(id)
            .frame(egui::Frame::NONE)
            .desired_width((inner.width() - 4.0).max(20.0)),
    )
}

/// สถานะของช่องกรอก — ใช้แสดงผลตรวจสอบแบบสด (ดีไซน์อ้างอิง: Live Validation)
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum FieldStatus {
    /// ไม่มีสถานะ
    #[default]
    None,
    /// ถูกต้อง (เขียว)
    Valid(String),
    /// ผิดพลาด (แดง)
    Invalid(String),
}

/// ความสูงของช่องกรอก
const FIELD_HEIGHT: f32 = 34.0;
/// ความสูงที่เผื่อไว้ให้ป้ายลอย
const FIELD_LABEL_HEIGHT: f32 = 16.0;
/// ความสูงแถวข้อความแจ้งเตือน
const FIELD_STATUS_HEIGHT: f32 = 17.0;

/// วาดกรอบช่องกรอก + เงาเรืองแสงตอนโฟกัส
fn paint_field(ui: &Ui, rect: Rect, focused: bool) {
    if focused {
        // วาดวงนอกก่อน (จางที่สุด) แล้วค่อยวงใน (เข้ม) → ได้ความรู้สึกเหมือนเรืองแสง
        ui.painter().rect_stroke(
            rect.expand(6.0),
            radius::MD + 6.0,
            Stroke::new(4.0, Palette::ACCENT_GLOW_FAINT),
            egui::StrokeKind::Outside,
        );
        ui.painter().rect_stroke(
            rect.expand(2.0),
            radius::MD + 2.0,
            Stroke::new(3.0, Palette::ACCENT_GLOW),
            egui::StrokeKind::Outside,
        );
    }
    ui.painter().rect_filled(rect, radius::MD, Palette::FIELD);
    ui.painter().rect_stroke(
        rect,
        radius::MD,
        Stroke::new(
            if focused { 1.5 } else { 1.0 },
            if focused {
                Palette::ACCENT
            } else {
                Palette::BORDER
            },
        ),
        egui::StrokeKind::Inside,
    );
}

/// ช่องกรอกสไตล์ดีไซน์อ้างอิง — ป้ายลอย + ขอบเรืองแสง + ผลตรวจสอบแบบสด
///
/// - **ป้ายลอย (Floating Label)** — ตอนช่องยังว่างและยังไม่โฟกัส ป้ายจะอยู่ในช่องเป็น
///   placeholder พอโฟกัสหรือพิมพ์แล้ว ป้ายจะลอยขึ้นไปเกาะขอบบน (เจาะช่องว่างในเส้นขอบ)
/// - **ขอบเรืองแสง (Glow Border)** — ตอนโฟกัส ขอบจะเรืองแสงสีหลัก
/// - **ตรวจสอบสด (Live Validation)** — ✓ เขียวเมื่อถูกต้อง / ✕ แดงพร้อมคำอธิบาย
pub fn labeled_field(
    ui: &mut Ui,
    label: &str,
    text: &mut String,
    width: f32,
    status: &FieldStatus,
) -> Response {
    let id = ui.scope_id().with(("mackeys_field", label));
    let focused = ui.memory(|m| m.has_focus(id));
    let has_text = !text.is_empty();

    // จองพื้นที่ทั้งหมด: แถวป้ายลอย + ช่องกรอก + แถวข้อความแจ้งเตือน
    let (outer, _) = ui.allocate_exact_size(
        Vec2::new(
            width,
            FIELD_LABEL_HEIGHT + FIELD_HEIGHT + FIELD_STATUS_HEIGHT,
        ),
        Sense::hover(),
    );
    let field_rect = Rect::from_min_max(
        egui::pos2(outer.left(), outer.top() + FIELD_LABEL_HEIGHT),
        egui::pos2(outer.right(), outer.top() + FIELD_LABEL_HEIGHT + FIELD_HEIGHT),
    );

    paint_field(ui, field_rect, focused);

    // ช่องข้อความ
    let inner = field_rect.shrink2(Vec2::new(10.0, 8.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let resp = child.add(
        egui::TextEdit::singleline(text)
            .id(id)
            .frame(egui::Frame::NONE)
            .desired_width((inner.width() - 24.0).max(20.0))
            .hint_text(if has_text { "" } else { label }),
    );

    // ป้ายลอย — โผล่ขึ้นมาเกาะขอบบนเมื่อโฟกัสหรือพิมพ์แล้ว
    if focused || has_text {
        let font_id = egui::FontId::new(font::TINY, egui::FontFamily::Proportional);
        let size = ui
            .painter()
            .layout_no_wrap(label.to_string(), font_id.clone(), Palette::TEXT)
            .size();
        let label_rect = Rect::from_min_max(
            egui::pos2(field_rect.left() + 12.0, field_rect.top() - 7.0),
            egui::pos2(
                field_rect.left() + 12.0 + size.x + 10.0,
                field_rect.top() + 8.0,
            ),
        );
        // เจาะช่องว่างในเส้นขอบ ให้ป้ายดูเหมือนลอยอยู่บนขอบจริง ๆ
        ui.painter().rect_filled(label_rect, 3.0, Palette::FIELD);
        ui.painter().text(
            label_rect.left_center() + Vec2::new(5.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            font_id,
            if focused {
                Palette::ACCENT_HI
            } else {
                Palette::TEXT_FAINT
            },
        );
    }

    // ผลตรวจสอบแบบสด
    let (icon, color, msg) = match status {
        FieldStatus::None => (None, Palette::TEXT_FAINT, ""),
        FieldStatus::Valid(m) => (Some("✓"), Palette::OK, m.as_str()),
        FieldStatus::Invalid(m) => (Some("✕"), Palette::DANGER, m.as_str()),
    };
    if let Some(icon) = icon {
        ui.painter().text(
            egui::pos2(field_rect.right() - 16.0, field_rect.center().y),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::new(font::BODY, egui::FontFamily::Proportional),
            color,
        );
        if !msg.is_empty() {
            ui.painter().text(
                egui::pos2(outer.left() + 1.0, outer.bottom() - 3.0),
                egui::Align2::LEFT_BOTTOM,
                msg,
                egui::FontId::new(font::TINY, egui::FontFamily::Proportional),
                color,
            );
        }
    }

    resp
}

/// ตัวช่วยจัดกึ่งกลางแนวตั้ง
pub fn centered(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.with_layout(egui::Layout::top_down(Align::Center), add);
}

#[cfg(test)]
mod tests {
    use super::{display_keys, layout, logo_dot_offsets, logo_dot_radius, space};

    /// ในโหมด Mac ผู้ใช้กด `Alt` (ซ้าย) ไม่ใช่ `Ctrl` → ต้องแสดงเป็น ALT
    #[test]
    fn mac_mode_shows_alt_instead_of_ctrl() {
        assert_eq!(display_keys("ctrl+shift+3", true), "alt+shift+3");
        assert_eq!(display_keys("ctrl+tab", true), "alt+tab");
        assert_eq!(display_keys("ctrl+q", true), "alt+q");
        assert_eq!(display_keys("ctrl+`", true), "alt+`");
    }

    /// โหมดปกติ (ไม่ใช่ Mac) → แสดงตามจริง
    #[test]
    fn normal_mode_keeps_ctrl() {
        assert_eq!(display_keys("ctrl+shift+3", false), "ctrl+shift+3");
        assert_eq!(display_keys("alt+space", false), "alt+space");
    }

    /// ปุ่มที่ไม่ใช่ ctrl ต้องไม่ถูกแตะ (เช่น alt+space ที่ผู้ใช้ตั้งเอง)
    #[test]
    fn other_keys_are_untouched() {
        assert_eq!(display_keys("alt+space", true), "alt+space");
        assert_eq!(display_keys("win+space", true), "win+space");
        // ตัวพิมพ์ใหญ่/เล็กก็ต้องถูกแปลง
        assert_eq!(display_keys("CTRL+F5", true), "alt+F5");
    }

    /// ค่าเลย์เอาต์ทุกตัวต้องเป็นพหุลของ 4 — จะได้เข้ากับ spacing scale เดียวกัน
    #[test]
    fn layout_tokens_follow_the_spacing_scale() {
        for v in [
            layout::NAV_PAD_X,
            layout::NAV_PAD_Y,
            layout::NAV_ITEM_HEIGHT,
            layout::TOP_BAR_HEIGHT,
            layout::CONTENT_PAD_X,
            layout::CONTENT_PAD_Y,
            layout::CARD_GAP_Y,
        ] {
            assert_eq!(v % 4.0, 0.0, "ค่าเลย์เอาต์ {v} ต้องเป็นพหุลของ 4");
        }
    }

    /// รายการเมนูต้องมีระยะห่างเท่ากันทั้งสองข้าง
    /// (เคยชิดเส้นแบ่งขวาพอดี เพราะขอบซ้าย 8 px ขอบขวา 8 px แต่ไฮไลต์โดนตัด)
    #[test]
    fn nav_item_keeps_padding_on_both_sides() {
        let item_w = layout::nav_item_width(layout::NAV_WIDTH);
        assert_eq!(item_w, 200.0);
        assert_eq!(
            layout::NAV_WIDTH - item_w,
            2.0 * layout::NAV_PAD_X,
            "รายการเมนูต้องมีระยะห่างเท่ากันทั้งซ้ายและขวา"
        );
    }

    /// หัวข้อหน้ากับขอบซ้ายของการ์ดต้องตรงกันเป๊ะ
    /// (เคยหัวข้อชิดแถบเมนู ส่วนการ์ดเว้นอีก 12 px → ดูไม่เรียบร้อย)
    #[test]
    fn heading_and_cards_share_the_same_left_edge() {
        assert_eq!(
            layout::card_left_edge(layout::NAV_WIDTH),
            layout::content_left_edge(layout::NAV_WIDTH),
            "การ์ดต้องเริ่มตรงกับหัวข้อหน้า"
        );
        assert_eq!(layout::content_left_edge(layout::NAV_WIDTH), 248.0);
    }

    /// พื้นที่เนื้อหาต้องมีที่หายใจจากแถบเมนู และคำนวณความกว้างได้ตรง
    #[test]
    fn content_has_breathing_room_from_the_side_menu() {
        assert!(layout::CONTENT_PAD_X >= space::XL);
        assert_eq!(
            layout::content_width(900.0),
            900.0 - layout::NAV_WIDTH - 2.0 * layout::CONTENT_PAD_X
        );
        // ความกว้างต้องไม่ติดลบแม้หน้าต่างจะแคบมาก
        assert_eq!(layout::content_width(100.0), 0.0);
    }

    /// จุด 3 จุดในโลโก้ต้องไม่ซ้อนกัน (เคยซ้อนกันจนกลายเป็น "ขีด" เส้นเดียว)
    #[test]
    fn logo_dots_do_not_overlap() {
        for size in [32.0_f32, 54.0] {
            let offsets = logo_dot_offsets(size);
            let r = logo_dot_radius(size);
            let gap = (offsets[1] - offsets[0]).abs();
            assert!(
                gap > r * 2.0,
                "จุดในโลโก้ขนาด {size} ต้องไม่ทับกัน (ระยะ {gap} ต้องมากกว่า {0})",
                r * 2.0
            );
            // ทั้งสามจุดต้องอยู่ในกรอบโลโก้
            let total = offsets[2] + r;
            assert!(total < size / 2.0, "จุดต้องอยู่ในโลโก้ (โดนขอบ {total})");
        }
    }
}
