use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};

use egui::{Margin, Align, Color32, ComboBox, RichText, Stroke, Vec2};

use crate::action::{Action, ScreenshotMode};
use crate::config::Config;
use crate::engine::{AppEvent, BindingStatus, Runtime, WinCommand};
use crate::ui::{self, Palette};

/// เก็บ egui context ไว้ให้ thread อื่นเรียก request_repaint เพื่อปลุก GUI
static CONTEXT: OnceLock<Mutex<Option<egui::Context>>> = OnceLock::new();

/// โหลดฟอนต์ที่รองรับภาษาไทย — ฟอนต์เริ่มต้นของ egui ไม่มีอักษรไทย (จะขึ้นเป็นกล่อง)
pub fn setup_fonts(ctx: &egui::Context) {
    ctx.set_fonts(build_font_definitions());
    ui::apply_style(ctx);
}

/// สร้างชุดฟอนต์ของแอป — แยกเป็นฟังก์ชันบริสุทธิ์เพื่อให้เขียนเทสต์ได้
///
/// ลำดับสำคัญ:
/// 1. `Tahoma` — อักษรไทย + ละติน (ใช้เป็นตัวหลัก)
/// 2. `Segoe UI Symbol` — สัญลักษณ์พิเศษ เช่น ⌘ (U+2318) ที่ Tahoma ไม่มี
/// 3. `Segoe UI Emoji` — อีโมจิ
pub fn build_font_definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();

    // ฟอนต์หลัก (ไทย + ละติน) ต้องมาก่อน
    let primary = [
        ("Tahoma", r"C:\Windows\Fonts\tahoma.ttf"),
        ("Microsoft Sans Serif", r"C:\Windows\Fonts\micross.ttf"),
        ("Leelawadee UI", r"C:\Windows\Fonts\Leelawadeeui.ttf"),
        ("Cordia", r"C:\Windows\Fonts\cordia.ttc"),
        ("Angsana New", r"C:\Windows\Fonts\angsana.ttc"),
    ];
    // ฟอนต์สำรองสำหรับสัญลักษณ์แปลก ๆ (⌘ ★ ◆) ที่ Tahoma ไม่มี
    let symbols = [
        ("Segoe UI Symbol", r"C:\Windows\Fonts\seguisym.ttf"),
        ("Segoe UI Emoji", r"C:\Windows\Fonts\seguiemj.ttf"),
    ];

    for (name, path) in primary {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        fonts
            .font_data
            .insert(name.to_string(), egui::FontData::from_owned(bytes).into());
        // ใส่ฟอนต์ไทยไว้ "หน้าสุด" ของครอบครัว → อักษรไทยมาจาก Tahoma
        // ส่วนสัญลักษณ์แปลก ๆ จะ fallback ไปฟอนต์ถัดไป
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, name.to_string());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, name.to_string());
        tracing::info!("โหลดฟอนต์ {name} เพื่อรองรับภาษาไทยแล้ว");
        break;
    }

    // ต่อท้ายด้วยฟอนต์สัญลักษณ์ (fallback สำหรับอักขระที่ Tahoma ไม่มี)
    for (name, path) in symbols {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        fonts
            .font_data
            .insert(name.to_string(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push(name.to_string());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push(name.to_string());
        tracing::info!("โหลดฟอนต์สัญลักษณ์ {name}");
    }

    fonts
}

pub fn init_context(ctx: egui::Context) {
    let _ = CONTEXT.set(Mutex::new(Some(ctx)));
}

/// ปลุก GUI ให้มาอ่าน event ใหม่ (จำเป็นตอนหน้าต่างถูกซ่อนอยู่)
pub fn wake() {
    if let Some(slot) = CONTEXT.get()
        && let Ok(guard) = slot.lock()
        && let Some(ctx) = guard.as_ref()
    {
        ctx.request_repaint();
    }
}

/// หน้าที่อยู่ในเมนูด้านข้าง
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Overview,
    Shortcuts,
    Languages,
    System,
}

impl Page {
    const ALL: [Page; 4] = [
        Page::Overview,
        Page::Shortcuts,
        Page::Languages,
        Page::System,
    ];

    fn icon(self) -> ui::Icon {
        match self {
            Page::Overview => ui::Icon::Dashboard,
            Page::Shortcuts => ui::Icon::Keyboard,
            Page::Languages => ui::Icon::Globe,
            Page::System => ui::Icon::Sliders,
        }
    }

    /// เลือกหน้าเริ่มต้นจากตัวแปรแวดล้อย `MACKEYS_PAGE` (สำหรับ dev/ถ่ายภาพหน้าจอ)
    /// ค่าที่รับได้: `overview` (ค่าเริ่มต้น), `shortcuts`, `languages`, `system`
    fn from_env() -> Page {
        match std::env::var("MACKEYS_PAGE").as_deref() {
            Ok("shortcuts") => Page::Shortcuts,
            Ok("languages") => Page::Languages,
            Ok("system") => Page::System,
            _ => Page::Overview,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Page::Overview => "ภาพรวม",
            Page::Shortcuts => "คีย์ลัด",
            Page::Languages => "ภาษาอินพุต",
            Page::System => "ระบบ",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Page::Overview => "ภาพรวม",
            Page::Shortcuts => "คีย์ลัด",
            Page::Languages => "ภาษาอินพุต",
            Page::System => "ระบบ",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Page::Overview => "สถานะการทำงานและตัวเลือกหลัก",
            Page::Shortcuts => "กำหนดคีย์ลัดและสิ่งที่ต้องการให้ทำ",
            Page::Languages => "เลือกภาษาที่จะสลับไปมา",
            Page::System => "ตัวเลือกระบบ ไฟล์ตั้งค่า และการดูแลรักษา",
        }
    }
}

pub struct SettingsApp {
    rt: Arc<Runtime>,
    cmd_tx: Sender<WinCommand>,
    events: Receiver<AppEvent>,
    config_path: PathBuf,
    config: Config,
    status: Vec<BindingStatus>,
    layouts: Vec<String>,
    message: Option<String>,
    dirty: bool,
    pending_delete: Option<usize>,
    quitting: bool,
    first_frame: bool,
    /// หน้าที่เลือกอยู่
    page: Page,
    /// แถวคีย์ลัดที่กำลังแก้คีย์ (คลิกที่คีย์แคป)
    editing: Option<usize>,
    /// ข้อความที่กำลังแก้ในช่องคีย์ลัด — ต้องเก็บไว้ข้ามเฟรม
    /// (ถ้าเก็บในตัวแปรชั่วคราวของแต่ละเฟรม ข้อความที่พิมพ์จะถูกรีเซ็ตทุกเฟรม)
    edit_text: String,
    /// ข้อความแจ้งเตือนชั่วคราว
    toast: Option<(String, bool)>,
}

impl SettingsApp {
    pub fn new(
        rt: Arc<Runtime>,
        cmd_tx: Sender<WinCommand>,
        events: Receiver<AppEvent>,
        config_path: PathBuf,
    ) -> Self {
        let config = rt.config.read().map(|c| c.clone()).unwrap_or_default();
        let status = rt.status.read().map(|s| s.clone()).unwrap_or_default();
        let layouts = crate::win::layout::installed()
            .into_iter()
            .map(|l| l.name)
            .collect();

        // ตัวแปรแวดล้อย `MACKEYS_EDIT` = เลขแถวที่ต้องการเปิดโหมดแก้ไขคีย์ลัดทันที
        // (ใช้ตอน dev/ถ่ายภาพหน้าจอเท่านั้น)
        let edit_index = std::env::var("MACKEYS_EDIT")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|i| *i < config.bindings.len());
        let edit_text = edit_index
            .and_then(|i| config.bindings.get(i).map(|b| b.keys.clone()))
            .unwrap_or_default();

        Self {
            rt,
            cmd_tx,
            events,
            config_path,
            config,
            status,
            layouts,
            message: None,
            dirty: false,
            pending_delete: None,
            quitting: false,
            first_frame: true,
            page: Page::from_env(),
            editing: edit_index,
            edit_text,
            toast: None,
        }
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                AppEvent::ShowSettings => {
                    ctx.send_viewport_cmd_to(
                        egui::ViewportId::ROOT,
                        egui::ViewportCommand::Visible(true),
                    );
                }
                AppEvent::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    // กันเหนือ: ถ้า GUI ไม่ยอมปิดในเวลา 2 วินาที ให้จบโปรแกรมเลย
                    std::thread::spawn(|| {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        std::process::exit(0);
                    });
                }
                AppEvent::StatusChanged => {
                    self.status = self.rt.status.read().map(|s| s.clone()).unwrap_or_default();
                }
                AppEvent::Notify(msg) => {
                    self.message = Some(msg);
                }
            }
        }
    }

    fn save(&mut self) {
        match crate::config::save(&self.config_path, &self.config) {
            Ok(()) => {
                if let Ok(mut cfg) = self.rt.config.write() {
                    *cfg = self.config.clone();
                }
                let _ = self.cmd_tx.send(WinCommand::ReloadConfig);
                self.message = Some("บันทึกแล้ว — นำคีย์ลัดใหม่มาใช้ทันที".into());
                self.dirty = false;
            }
            Err(e) => self.message = Some(format!("บันทึกไม่สำเร็จ: {e}")),
        }
    }

    fn reload_from_disk(&mut self) {
        match crate::config::load(&self.config_path) {
            Ok(cfg) => {
                self.config = cfg;
                self.message = Some("โหลดจากไฟล์แล้ว (ยังไม่บันทึก)".into());
            }
            Err(e) => self.message = Some(format!("โหลดไม่สำเร็จ: {e}")),
        }
    }

    /// จำนวนคีย์ลัดที่ลงทะเบียนสำเร็จ / มีปัญหา
    fn counts(&self) -> (usize, usize) {
        let ok = self
            .status
            .iter()
            .filter(|s| s.ok && s.message.is_none())
            .count();
        let bad = self.status.iter().filter(|s| !s.ok).count();
        (ok, bad)
    }

    fn set_page(&mut self, page: Page) {
        self.page = page;
        self.editing = None;
    }
}

impl eframe::App for SettingsApp {
    /// ถูกเรียกแม้ตอนหน้าต่างถูกซ่อน — ใช้รับคำสั่งจาก tray
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.drain_events(&ctx);

        // eframe จะแสดงหน้าต่างหลังเฟรมแรกเสมอ — ซ่อนกลับไปเพื่อให้เป็นโปรแกรม tray
        // (ยกเว้นตอน dev: ตั้ง MACKEYS_SHOW_UI=1 เพื่อให้หน้าต่างเปิดขึ้นมาเลย)
        if self.first_frame {
            self.first_frame = false;
            if std::env::var_os("MACKEYS_SHOW_UI").is_none() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        // กดปิดหน้าต่าง = ซ่อนไว้ ไม่ใช่ออกจากโปรแกรม (ยกเว้นตอนสั่งออกจาก tray)
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        self.top_bar(ui);
        self.side_nav(ui);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new().fill(Palette::BG).inner_margin(Margin {
                    left: ui::layout::CONTENT_PAD_X as i8,
                    right: ui::layout::CONTENT_PAD_X as i8,
                    top: ui::layout::CONTENT_PAD_Y as i8,
                    bottom: ui::layout::CONTENT_PAD_Y as i8,
                }),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        match self.page {
                            Page::Overview => self.page_overview(ui),
                            Page::Shortcuts => self.page_shortcuts(ui),
                            Page::Languages => self.page_languages(ui),
                            Page::System => self.page_system(ui),
                        }
                        ui.add_space(ui::space::XXL);
                    });
            });
    }
}

impl SettingsApp {
    // ─────────────────────────── แถบบน ───────────────────────────
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar")
            .exact_size(ui::layout::TOP_BAR_HEIGHT)
            .frame(
                egui::Frame::new()
                    .fill(Palette::NAV)
                    .stroke(Stroke::new(1.0, Palette::BORDER))
                    // ขอบซ้าย/ขวา 24 px (เท่าพื้นที่เนื้อหา) — โลโก้ต้องไม่ชิดขอบหน้าต่าง
                    .inner_margin(Margin::symmetric(ui::space::XL as i8, 0)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui::logo(ui, 32.0);
                    ui.add_space(ui::space::MD);
                    ui.label(
                        RichText::new("MacKeys")
                            .size(ui::font::TITLE_LG)
                            .strong()
                            .color(Palette::TEXT),
                    );
                    ui.add_space(ui::space::MD);
                    ui.label(
                        RichText::new("คีย์ลัดสไตล์ macOS สำหรับ Windows")
                            .size(ui::font::SMALL)
                            .color(Palette::TEXT_FAINT),
                    );

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if self.dirty {
                            ui::pill(ui, "มีการเปลี่ยนแปลง", Palette::WARN, Palette::WARN_DIM);
                            ui.add_space(ui::space::SM);
                        }
                        let enabled = self.dirty;
                        let btn = egui::Button::new(
                            RichText::new("บันทึก")
                                .size(ui::font::BODY_LG)
                                .strong()
                                .color(if enabled {
                                    Color32::WHITE
                                } else {
                                    Palette::TEXT_FAINT
                                }),
                        )
                        .fill(if enabled {
                            Palette::ACCENT
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::new(1.0, Palette::BORDER))
                        .corner_radius(egui::CornerRadius::same(ui::radius::LG as u8))
                        .min_size(egui::Vec2::new(92.0, 34.0));
                        if ui.add_enabled(enabled, btn).clicked() {
                            self.save();
                        }
                    });
                });
            });
    }

    // ─────────────────────── เมนูด้านข้าง ───────────────────────
    fn side_nav(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("nav")
            .exact_size(ui::layout::NAV_WIDTH)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Palette::NAV)
                    .stroke(Stroke::new(1.0, Palette::BORDER))
                    // ขอบซ้าย/ขวา 12 px + ขอบบน/ล่าง 16 px
                    // → รายการที่เลือกไม่ชิดเส้นแบ่ง และไม่ชิดขอบหน้าต่าง
                    .inner_margin(Margin {
                        left: ui::layout::NAV_PAD_X as i8,
                        right: ui::layout::NAV_PAD_X as i8,
                        top: ui::layout::NAV_PAD_Y as i8,
                        bottom: ui::layout::NAV_PAD_Y as i8,
                    }),
            )
            .show(ui, |ui| {
                for page in Page::ALL {
                    let selected = self.page == page;
                    let resp = ui::nav_item(ui, page.label(), page.icon(), page.label(), selected);
                    if resp.clicked() {
                        self.set_page(page);
                    }
                    ui.add_space(2.0);
                }

                // ดันกล่องสถานะ + เวอร์ชันไปลงล่างสุดของแถบ
                //
                // หมายเหตุ: ไม่ใช้ `Layout::bottom_up` เพราะ `egui::Frame::show`
                // จะขยายเต็มพื้นที่ที่เหลือของเลย์เอาต์แบบ bottom_up
                // (การ์ดจะสูงเท่าทั้งแถบ) — จึงเว้นที่เองแล้ววาดในพื้นที่ที่จองไว้
                const NAV_BOTTOM_BLOCK_H: f32 = 124.0;
                let avail_h = ui.available_height();
                ui.add_space((avail_h - NAV_BOTTOM_BLOCK_H).max(0.0));

                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), NAV_BOTTOM_BLOCK_H),
                    egui::Layout::top_down(Align::Min),
                    |ui| {
                        // กล่องสรุปสถานะด้านล่าง
                        ui::card(ui, |ui| {
                            let (ok, bad) = self.counts();
                            ui.label(
                                RichText::new("สถานะคีย์ลัด")
                                    .size(ui::font::SMALL)
                                    .color(Palette::TEXT_FAINT),
                            );
                            ui.add_space(ui::space::XS);
                            ui.horizontal(|ui| {
                                ui::pill(ui, &format!("พร้อม {ok}"), Palette::OK, Palette::OK_DIM);
                                if bad > 0 {
                                    ui::pill(
                                        ui,
                                        &format!("มีปัญหา {bad}"),
                                        Palette::DANGER,
                                        Palette::DANGER_DIM,
                                    );
                                }
                            });
                        });

                        ui.add_space(ui::space::SM);
                        ui.label(
                            RichText::new("MacKeys v0.1.0")
                                .size(ui::font::TINY)
                                .color(Palette::TEXT_FAINT),
                        );
                    },
                );
            });
    }

    // ───────────────────────── หน้าภาพรวม ─────────────────────────
    fn page_overview(&mut self, ui: &mut egui::Ui) {
        self.page_heading(ui);

        // ── การ์ดหลัก ──
        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui::logo(ui, 54.0);
                ui.add_space(ui::space::LG);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("MacKeys")
                            .size(ui::font::DISPLAY)
                            .strong()
                            .color(Palette::TEXT),
                    );
                    ui.label(
                        RichText::new("ทำให้คีย์ลัดบน Windows ใช้งานเหมือน macOS")
                            .size(ui::font::BODY)
                            .color(Palette::TEXT_DIM),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let mut enabled = self.config.general.enabled;
                    if ui::toggle(ui, &mut enabled).changed() {
                        self.config.general.enabled = enabled;
                        self.dirty = true;
                    }
                    ui.add_space(ui::space::SM);
                    ui.label(
                        RichText::new("เปิดใช้งานทั้งหมด")
                            .size(ui::font::BODY)
                            .color(Palette::TEXT),
                    );
                });
            });
        });

        ui.add_space(ui::space::MD);

        // ── สถิติ ──
        {
            let (ok, bad) = self.counts();
            let total = self.config.bindings.len();
            let tiles: Vec<(&str, String, Color32, &str)> = vec![
                ("คีย์ลัดทั้งหมด", total.to_string(), Palette::TEXT, "ทั้งหมด"),
                ("พร้อมใช้งาน", ok.to_string(), Palette::OK, "ลงทะเบียนสำเร็จ"),
                (
                    "มีปัญหา",
                    bad.to_string(),
                    if bad > 0 { Palette::DANGER } else { Palette::TEXT_DIM },
                    "ชนกับโปรแกรมอื่น",
                ),
            ];
            // `columns` หารความกว้างเท่ากันให้ทุกกล่อง (เดิมใช้ available_width
            // ซึ่งเปลี่ยนไปเรื่อย ๆ ทำให้กล่องไม่เท่ากัน)
            ui.scope(|ui| {
                // เว้นระหว่างกล่องสถิติให้พอกำลังหายใจ (ค่าเดิม 8 px แค่เนียนไป)
                ui.spacing_mut().item_spacing.x = ui::space::LG;
                ui.columns(3, |cols| {
                    for (col, (title, value, color, note)) in cols.iter_mut().zip(tiles.iter()) {
                        col.vertical(|ui| self.stat_tile(ui, title, value, *color, note));
                    }
                });
            });
        }

        ui.add_space(ui::space::MD);

        // ── โหมด Mac ──
        ui::titled_card(
            ui,
            "โหมด Mac",
            Some("ปุ่ม Alt ฝั่งซ้ายทำหน้าที่เป็น ⌘ ของ macOS"),
            |ui| {
                ui.horizontal(|ui| {
                    let mut mac = self.config.mac.enabled;
                    if ui::toggle(ui, &mut mac).changed() {
                        self.config.mac.enabled = mac;
                        self.dirty = true;
                    }
                    ui.add_space(ui::space::SM);
                    ui.label(
                        RichText::new("ให้ปุ่ม Alt ฝั่งซ้ายทำหน้าที่เป็น Ctrl")
                            .size(ui::font::BODY)
                            .color(Palette::TEXT),
                    );
                });

                ui.add_space(ui::space::MD);
                // แบ่งตัวอย่างเป็นแถวละ 3 ชิ้น (กำหนดเอง ไม่ใช้การตัดบรรทัดอัตโนมัติ)
                // → ไม่ล้นออกนอกการ์ด และคีย์แคปไม่ถูกแยกจากคำอธิบาย
                let samples = [
                    ("alt+c", "คัดลอก"),
                    ("alt+v", "วาง"),
                    ("alt+z", "ย้อนกลับ"),
                    ("alt+space", "สลับภาษา"),
                    ("alt+tab", "สลับแอป"),
                ];
                for row in samples.chunks(3) {
                    ui.horizontal(|ui| {
                        for (keys, desc) in row {
                            ui::keycap_with_label(ui, keys, desc);
                            ui.add_space(ui::space::LG);
                        }
                    });
                    ui.add_space(ui::space::SM);
                }

                ui.add_space(ui::space::SM);
                ui.label(
                    RichText::new("ปุ่ม Alt ฝั่งขวายังเป็น Alt จริง (ใช้ Alt+Tab / Alt+F4 แบบเดิมได้)")
                        .size(ui::font::SMALL)
                        .color(Palette::TEXT_FAINT),
                );
            },
        );

        ui.add_space(ui::space::MD);

        // ── ทางลัด ──
        ui::titled_card(ui, "ทางลัด", None, |ui| {
            ui.horizontal(|ui| {
                if ui::primary_button_w(ui, "สลับภาษาตอนนี้", 200.0).clicked() {
                    let _ = self
                        .rt
                        .actions
                        .send(Action::SwitchLayout("cycle".into()));
                }
                ui.add_space(ui::space::SM);
                if ui::ghost_button_w(ui, "รีเซ็ตปุ่มที่ค้าง (Ctrl/Alt)", 200.0).clicked() {
                    let _ = self.cmd_tx.send(WinCommand::ReleaseKeys);
                    self.toast = Some(("รีเซ็ตปุ่มที่ค้างแล้ว".into(), true));
                }
                ui.add_space(ui::space::SM);
                if ui::ghost_button_w(ui, "เปิดโฟลเดอร์ config", 200.0).clicked() {
                    self.open_config_dir();
                }
            });
            ui.add_space(ui::space::SM);
            ui.label(
                RichText::new("ถ้าพิมพ์ไม่ได้หรือคีย์ลัดไม่ทำงาน ให้กด \"รีเซ็ตปุ่มที่ค้าง\" ก่อน")
                    .size(ui::font::SMALL)
                    .color(Palette::TEXT_FAINT),
            );
        });

        self.show_toast(ui);
    }

    fn stat_tile(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        value: &str,
        color: Color32,
        note: &str,
    ) {
        ui::card(ui, |ui| {
            ui.label(
                RichText::new(title)
                    .size(ui::font::SMALL)
                    .color(Palette::TEXT_FAINT),
            );
            ui.add_space(ui::space::XS);
            ui.label(RichText::new(value).size(ui::font::STAT).strong().color(color));
            ui.add_space(ui::space::XS);
            ui.label(RichText::new(note).size(ui::font::TINY).color(Palette::TEXT_FAINT));
        });
    }

    // ───────────────────────── หน้าคีย์ลัด ─────────────────────────
    fn page_shortcuts(&mut self, ui: &mut egui::Ui) {
        self.page_heading(ui);

        if self.config.bindings.is_empty() {
            ui::card(ui, |ui| {
                ui::centered(ui, |ui| {
                    ui.add_space(ui::space::MD);
                    ui.label(
                        RichText::new("ยังไม่มีคีย์ลัด")
                            .size(15.0)
                            .color(Palette::TEXT_DIM),
                    );
                    ui.label(
                        RichText::new("กดปุ่มด้านล่างเพื่อเพิ่มคีย์ลัดแรก")
                            .size(12.0)
                            .color(Palette::TEXT_FAINT),
                    );
                    ui.add_space(ui::space::MD);
                });
            });
        }

        for i in 0..self.config.bindings.len() {
            self.binding_row(ui, i);
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui::primary_button(ui, "+  เพิ่มคีย์ลัด").clicked() {
                self.config.bindings.push(crate::config::Binding {
                    keys: "ctrl+shift+3".into(),
                    action: Action::Screenshot(ScreenshotMode::Full),
                    enabled: true,
                });
                self.dirty = true;
            }
        });

        self.show_toast(ui);
    }

    fn binding_row(&mut self, ui: &mut egui::Ui, i: usize) {
        let keys = self.config.bindings[i].keys.clone();
        let action = self.config.bindings[i].action.clone();
        let enabled = self.config.bindings[i].enabled;
        let status = self.status.get(i).cloned();
        // ในโหมด Mac ผู้ใช้กด "Alt" (ซ้าย) ไม่ใช่ "Ctrl" → แสดงปุ่มที่กดจริง
        let mac_mode = self.config.mac.enabled;
        let shown_keys = ui::display_keys(&keys, mac_mode);

        ui::card(ui, |ui| {
            ui.horizontal(|ui| {
                // สวิตช์เปิด/ปิด
                let mut on = enabled;
                if ui::toggle(ui, &mut on).changed() {
                    self.config.bindings[i].enabled = on;
                    self.dirty = true;
                }
                ui.add_space(ui::space::SM);

                // ── คีย์ลัด (แสดงเป็นคีย์แคป) ──
                // ใส่ความกว้างคงที่ เพื่อให้ช่องเลือกการกระทำของทุกแถวตรงกัน
                ui.vertical(|ui| {
                    ui.set_width(ui::KEYCAP_COL_WIDTH);
                    let resp = ui
                        .horizontal(|ui| {
                            ui::keycap_combo(ui, &shown_keys);
                        })
                        .response;
                    let resp = resp.on_hover_text("คลิกเพื่อแก้คีย์ลัด");
                    if resp.clicked() {
                        self.editing = Some(i);
                        self.edit_text = keys.clone();
                    }
                });
                ui.add_space(ui::space::MD);

                // ── การกระทำ ──
                match action.clone() {
                    Action::SwitchLayout(target) => {
                        let layouts = self.layouts.clone();
                        ComboBox::new(("layout", i), "")
                            .width(ui::COMBO_WIDTH)
                            .selected_text(layout_target_label(&target))
                            .show_ui(ui, |ui| {
                                let mut new_target = target.clone();
                                ui.selectable_value(
                                    &mut new_target,
                                    "cycle".to_string(),
                                    "ตามรายการที่ตั้งไว้",
                                );
                                ui.selectable_value(
                                    &mut new_target,
                                    "next".to_string(),
                                    "ภาษาถัดไปที่ติดตั้ง",
                                );
                                for name in &layouts {
                                    ui.selectable_value(
                                        &mut new_target,
                                        name.clone(),
                                        name.clone(),
                                    );
                                }
                                if new_target != target {
                                    self.config.bindings[i].action =
                                        Action::SwitchLayout(new_target);
                                    self.dirty = true;
                                }
                            });
                    }
                    Action::Screenshot(mode) => {
                        ComboBox::new(("shot", i), "")
                            .width(ui::COMBO_WIDTH)
                            .selected_text(mode_label(mode))
                            .show_ui(ui, |ui| {
                                let mut new_mode = mode;
                                ui.selectable_value(
                                    &mut new_mode,
                                    ScreenshotMode::Full,
                                    "จับทั้งจอ",
                                );
                                ui.selectable_value(
                                    &mut new_mode,
                                    ScreenshotMode::Region,
                                    "จับบริเวณ",
                                );
                                ui.selectable_value(
                                    &mut new_mode,
                                    ScreenshotMode::Panel,
                                    "เปิด Snipping Tool",
                                );
                                if new_mode != mode {
                                    self.config.bindings[i].action =
                                        Action::Screenshot(new_mode);
                                    self.dirty = true;
                                }
                            });
                    }
                    Action::AppSwitcher => {
                        ui.label(
                            RichText::new("สลับหน้าต่าง")
                                .size(ui::font::BODY)
                                .color(Palette::TEXT_DIM),
                        );
                    }
                    Action::SendKeys(target) => {
                        let mut keys = target.clone();
                        if ui::field(ui, &mut keys, 150.0).changed() {
                            self.config.bindings[i].action = Action::SendKeys(keys);
                            self.dirty = true;
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui::icon_button(ui, "×", "ลบคีย์ลัดนี้").clicked() {
                        self.pending_delete = Some(i);
                    }
                });
            });

            // ── แผงแก้ไขคีย์ลัด (แสดงตอนคลิกที่คีย์แคป) ──
            if self.editing == Some(i) {
                ui.add_space(ui::space::MD);
                self.edit_panel(ui, i);
            }

            // ── บรรทัดสถานะ ──
            // จัดเป็น 3 คอลัมน์ความกว้างคงที่ ให้ทุกแถวตรงกัน
            // (ก่อนหน้านี้ข้อความยืดตามความยาว → ชิดกันและคอลัมน์ข้ามแถวกัน)
            if let Some(st) = status {
                let (color, text) = match (&st.ok, &st.message) {
                    (true, None) => (Palette::OK, "พร้อมใช้งาน".to_string()),
                    (true, Some(m)) => (Palette::TEXT_FAINT, m.clone()),
                    (false, Some(m)) => (Palette::DANGER, m.clone()),
                    (false, None) => (Palette::TEXT_FAINT, "ไม่ทราบสถานะ".to_string()),
                };
                ui.horizontal(|ui| {
                    // เยื้องให้ตรงแนวกับคีย์แคป (สวิตช์ 42 px + ช่องว่าง)
                    ui.add_space(42.0 + ui::space::SM);

                    // คอลัมน์ 1: ปุ่มที่ใช้
                    ui::fixed_col(
                        ui,
                        ui::STATUS_KEYS_COL_WIDTH,
                        ui::STATUS_ROW_HEIGHT,
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("●").size(9.0).color(color));
                                ui.add_space(ui::space::XS);
                                ui.label(
                                    RichText::new(ui::display_keys(&st.keys, mac_mode))
                                        .size(ui::font::SMALL)
                                        .color(Palette::TEXT_FAINT),
                                );
                            });
                        },
                    );
                    ui.add_space(ui::space::LG);

                    // คอลัมน์ 2: คำอธิบาย action
                    ui::fixed_col(
                        ui,
                        ui::STATUS_ACTION_COL_WIDTH,
                        ui::STATUS_ROW_HEIGHT,
                        |ui| {
                            ui.label(
                                RichText::new(action.describe())
                                    .size(ui::font::SMALL)
                                    .color(Palette::TEXT_DIM),
                            );
                        },
                    );
                    ui.add_space(ui::space::LG);

                    // คอลัมน์ 3: สถานะ (กินพื้นที่ที่เหลือทั้งหมด)
                    ui.label(RichText::new(text).size(ui::font::SMALL).color(color));
                });
            }
        });

        if let Some(idx) = self.pending_delete.take()
            && idx < self.config.bindings.len()
        {
            self.config.bindings.remove(idx);
            self.dirty = true;
        }
    }

    /// แผงแก้ไขคีย์ลัด — ช่องกรอกสไตล์ดีไซน์อ้างอิง
    /// (ป้ายลอย + ขอบเรืองแสงตอนโฟกัส + ตรวจสอบคีย์ลัดแบบสด)
    fn edit_panel(&mut self, ui: &mut egui::Ui, i: usize) {
        // ตรวจคีย์ลัดใหม่ทุกครั้งที่พิมพ์ → ขึ้น ✓ เขียว / ✕ แดงทันที
        let status = shortcut_status(&self.edit_text);

        // ใช้ mem::take เพื่อไม่ให้ยืม `self` ซ้อนกันสองครั้ง
        let mut text = std::mem::take(&mut self.edit_text);
        ui::labeled_field(ui, "คีย์ลัด", &mut text, 240.0, &status);
        self.edit_text = text;

        ui.add_space(ui::space::SM);
        ui.horizontal(|ui| {
            if ui::primary_button(ui, "ตกลง").clicked() {
                let value = self.edit_text.trim().to_string();
                match crate::hotkey::Hotkey::parse(&value) {
                    Ok(_) => {
                        self.config.bindings[i].keys = value;
                        self.editing = None;
                        self.dirty = true;
                    }
                    Err(e) => {
                        self.toast = Some((format!("บันทึกไม่ได้: {e}"), false));
                    }
                }
            }
            ui.add_space(ui::space::SM);
            if ui::ghost_button(ui, "ยกเลิก").clicked() {
                self.editing = None;
            }
            ui.add_space(ui::space::MD);
            ui.label(
                RichText::new("ตัวอย่าง: ctrl+shift+3 · alt+space · win+`")
                    .size(ui::font::TINY)
                    .color(Palette::TEXT_FAINT),
            );
        });
    }

    // ──────────────────────── หน้าภาษาอินพุต ────────────────────────
    fn page_languages(&mut self, ui: &mut egui::Ui) {
        self.page_heading(ui);

        ui::titled_card(
            ui,
            "ภาษาที่ใช้สลับไปมา",
            Some("เรียงตามลำดับที่เลือก — เลือกภาษาให้เข้าชุดด้วย"),
            |ui| {
                let installed = self.layouts.clone();
                let cycle = self.config.layouts.cycle.clone();
                let mut changed = false;

                for name in &installed {
                    let in_cycle = cycle.contains(name);
                    let pos = cycle.iter().position(|c| c == name);
                    ui.horizontal(|ui| {
                        let mut on = in_cycle;
                        if ui::toggle(ui, &mut on).changed() {
                            if on {
                                self.config.layouts.cycle.push(name.clone());
                            } else {
                                self.config.layouts.cycle.retain(|c| c != name);
                            }
                            changed = true;
                        }
                        ui.add_space(ui::space::SM);
                        // ชื่อภาษาในคอลัมน์กว้างคงที่ → ให้ปุ่มเลื่อนลำดับตรงกันทุกแถว
                        ui::fixed_col(ui, 140.0, 28.0, |ui| {
                            ui.label(
                                RichText::new(name.clone())
                                    .size(ui::font::BODY_LG)
                                    .color(if in_cycle {
                                        Palette::TEXT
                                    } else {
                                        Palette::TEXT_FAINT
                                    }),
                            );
                        });

                        if let Some(p) = pos {
                            ui.add_space(ui::space::MD);
                            ui.label(
                                RichText::new(format!("ลำดับ {}", p + 1))
                                    .size(ui::font::SMALL)
                                    .color(Palette::TEXT_FAINT),
                            );
                            ui.add_space(ui::space::SM);
                            if ui::icon_button(ui, "↑", "ย้ายขึ้น").clicked() && p > 0 {
                                self.config.layouts.cycle.swap(p, p - 1);
                                changed = true;
                            }
                            if ui::icon_button(ui, "↓", "ย้ายลง").clicked()
                                && p + 1 < self.config.layouts.cycle.len()
                            {
                                self.config.layouts.cycle.swap(p, p + 1);
                                changed = true;
                            }
                        }
                    });
                    ui.add_space(ui::space::XS);
                }

                if changed {
                    self.dirty = true;
                }

                ui.add_space(ui::space::MD);
                let order = if self.config.layouts.cycle.is_empty() {
                    "ยังไม่ได้เลือกภาษาใด".to_string()
                } else {
                    self.config.layouts.cycle.join("  →  ")
                };
                ui.label(
                    RichText::new(format!("ลำดับการสลับ:  {order}"))
                        .size(ui::font::BODY)
                        .color(Palette::TEXT_DIM),
                );
            },
        );

        ui.add_space(ui::space::MD);

        ui::titled_card(
            ui,
            "ภาษาเริ่มต้น",
            Some("ภาษาที่จะถูกเลือกตอนเปิดโปรแกรม"),
            |ui| {
                let layouts = self.layouts.clone();
                let current = self
                    .config
                    .layouts
                    .default
                    .clone()
                    .unwrap_or_else(|| "(ไม่บังคับ)".to_string());
                ComboBox::new("default_layout", "")
                    .width(ui::COMBO_WIDTH)
                    .selected_text(current.clone())
                    .show_ui(ui, |ui| {
                        let mut value = self.config.layouts.default.clone();
                        ui.selectable_value(
                            &mut value,
                            None,
                            "(ไม่บังคับ)".to_string(),
                        );
                        for name in &layouts {
                            ui.selectable_value(&mut value, Some(name.clone()), name.clone());
                        }
                        if value != self.config.layouts.default {
                            self.config.layouts.default = value;
                            self.dirty = true;
                        }
                    });
            },
        );

        self.show_toast(ui);
    }

    // ─────────────────────────── หน้าระบบ ───────────────────────────
    fn page_system(&mut self, ui: &mut egui::Ui) {
        self.page_heading(ui);

        // ── ทั่วไป ──
        ui::titled_card(ui, "ทั่วไป", None, |ui| {
            ui.horizontal(|ui| {
                let mut enabled = self.config.general.enabled;
                if ui::toggle(ui, &mut enabled).changed() {
                    self.config.general.enabled = enabled;
                    self.dirty = true;
                }
                ui.label(
                    RichText::new("เปิดใช้งานโปรแกรม")
                        .size(ui::font::BODY)
                        .color(Palette::TEXT),
                );
            });

            ui.add_space(ui::space::MD);
            ui.horizontal(|ui| {
                let mut autostart = crate::autostart::is_installed();
                let resp = ui::toggle(ui, &mut autostart);
                if resp.changed() {
                    let result = if autostart {
                        crate::autostart::install()
                    } else {
                        crate::autostart::uninstall()
                    };
                    match result {
                        Ok(()) => {
                            self.toast = Some((
                                if autostart {
                                    "ตั้งให้รันพร้อม Windows แล้ว".into()
                                } else {
                                    "ยกเลิกการรันพร้อม Windows แล้ว".into()
                                },
                                true,
                            ));
                        }
                        Err(e) => {
                            self.toast = Some((format!("ทำไม่สำเร็จ: {e}"), false));
                        }
                    }
                }
                ui.label(
                    RichText::new("ให้โปรแกรมรันพร้อม Windows")
                        .size(ui::font::BODY)
                        .color(Palette::TEXT),
                );
            });

            ui.add_space(ui::space::MD);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("ระดับ log")
                        .size(ui::font::BODY)
                        .color(Palette::TEXT),
                );
                ui.add_space(ui::space::SM);
                let levels = ["error", "warn", "info", "debug", "trace"];
                let mut level = self.config.general.log_level.clone();
                ComboBox::new("log_level", "")
                    .width(ui::COMBO_WIDTH)
                    .selected_text(log_level_label(&level))
                    .show_ui(ui, |ui| {
                        for lv in levels {
                            ui.selectable_value(
                                &mut level,
                                lv.to_string(),
                                log_level_label(lv),
                            );
                        }
                    });
                if level != self.config.general.log_level {
                    self.config.general.log_level = level;
                    self.dirty = true;
                }
            });
            ui.add_space(ui::space::SM);
            ui.label(
                RichText::new("trace จะบันทึกทุกการกดคีย์ (ใช้ตอนวินิจฉัยปัญหา)")
                    .size(ui::font::TINY)
                    .color(Palette::TEXT_FAINT),
            );
        });

        ui.add_space(ui::space::MD);

        // ── ไฟล์ตั้งค่า ──
        ui::titled_card(ui, "ไฟล์ตั้งค่า", None, |ui| {
            ui.label(
                RichText::new(self.config_path.display().to_string())
                    .size(ui::font::SMALL)
                    .color(Palette::TEXT_FAINT),
            );
            ui.add_space(ui::space::MD);
            ui.horizontal(|ui| {
                if ui::ghost_button_w(ui, "เปิดโฟลเดอร์ config", 200.0).clicked() {
                    self.open_config_dir();
                }
                ui.add_space(ui::space::SM);
                if ui::ghost_button_w(ui, "โหลดใหม่จากไฟล์", 170.0).clicked() {
                    self.reload_from_disk();
                }
            });
        });

        ui.add_space(ui::space::MD);

        // ── ดูแลรักษา ──
        ui::card(ui, |ui| {
            ui.label(
                RichText::new("ดูแลรักษา")
                    .size(ui::font::TITLE)
                    .strong()
                    .color(Palette::TEXT),
            );
            ui.label(
                RichText::new("แก้ปัญหาคีย์ค้างหรือพิมพ์ไม่ได้")
                    .size(ui::font::SMALL)
                    .color(Palette::TEXT_FAINT),
            );
            ui.add_space(ui::space::MD);
            ui.horizontal(|ui| {
                if ui::ghost_button_w(ui, "รีเซ็ตปุ่มที่ค้าง (Ctrl/Alt)", 200.0).clicked() {
                    let _ = self.cmd_tx.send(WinCommand::ReleaseKeys);
                    self.toast = Some(("รีเซ็ตปุ่มที่ค้างแล้ว".into(), true));
                }
                ui.add_space(ui::space::SM);
                if ui::ghost_button_w(ui, "ตรวจสถานะปุ่ม", 200.0).clicked() {
                    self.toast = Some((
                        "ดูผลได้จากคำสั่ง mackeys diagnose-keys ใน PowerShell".into(),
                        true,
                    ));
                }
            });
        });

        self.show_toast(ui);
    }

    // ─────────────────────────── ส่วนร่วม ───────────────────────────
    fn page_heading(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(self.page.title())
                .size(ui::font::HEADING)
                .strong()
                .color(Palette::TEXT),
        );
        ui.label(
            RichText::new(self.page.subtitle())
                .size(ui::font::BODY)
                .color(Palette::TEXT_FAINT),
        );
        ui.add_space(ui::space::XL);

        if let Some(msg) = &self.message {
            ui::card(ui, |ui| {
                ui.label(
                    RichText::new(msg.clone())
                        .size(ui::font::BODY)
                        .color(Palette::ACCENT_HI),
                );
            });
            ui.add_space(ui::space::MD);
        }
    }

    fn show_toast(&mut self, ui: &mut egui::Ui) {
        if let Some((msg, ok)) = self.toast.take() {
            ui::card(ui, |ui| {
                ui.label(
                    RichText::new(msg)
                        .size(ui::font::BODY)
                        .color(if ok { Palette::OK } else { Palette::DANGER }),
                );
            });
            ui.add_space(ui::space::SM);
        }
    }

    fn open_config_dir(&mut self) {
        let dir = crate::config::dir_of(&self.config_path);
        if let Err(e) = crate::win::util::open_folder(&dir) {
            self.toast = Some((format!("เปิดโฟลเดอร์ไม่ได้: {e}"), false));
        }
    }
}

fn mode_label(mode: ScreenshotMode) -> &'static str {
    match mode {
        ScreenshotMode::Full => "จับทั้งจอ",
        ScreenshotMode::Region => "จับบริเวณ",
        ScreenshotMode::Panel => "เปิด Snipping Tool",
    }
}

/// ข้อความที่อ่านง่ายสำหรับระดับ log (ค่าใน config ยังเป็นชื่อระดับตามปกติ)
fn log_level_label(level: &str) -> &'static str {
    match level {
        "error" => "error — ผิดพลาดเท่านั้น",
        "warn" => "warn — มีคำเตือน",
        "info" => "info — ข้อมูลปกติ",
        "debug" => "debug — รายละเอียด",
        "trace" => "trace — ทุกการกดคีย์",
        _ => "info — ข้อมูลปกติ",
    }
}

/// สถานะของช่องคีย์ลัด (ใช้แสดงผลตรวจสอบแบบสดในแผงแก้ไข)
fn shortcut_status(text: &str) -> ui::FieldStatus {
    match crate::hotkey::Hotkey::parse(text.trim()) {
        Ok(_) => ui::FieldStatus::Valid("คีย์ลัดถูกต้อง".to_string()),
        Err(e) => ui::FieldStatus::Invalid(e),
    }
}

/// ข้อความที่อ่านง่ายสำหรับเป้าหมายการสลับภาษา
fn layout_target_label(target: &str) -> String {    match target {
        "cycle" => "ตามรายการที่ตั้งไว้".to_string(),
        "next" => "ภาษาถัดไปที่ติดตั้ง".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_font_definitions, layout_target_label, log_level_label, shortcut_status};
    use crate::ui;

    /// สร้าง context ที่โหลดฟอนต์ของแอปแล้ว (ต้องเรียก `run_ui` หนึ่งครั้งก่อน
    /// เพราะ egui สร้าง object ฟอนต์ตอนเริ่มเฟรมแรก)
    fn ctx_with_app_fonts() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_fonts(build_font_definitions());
        let _ = ctx.run_ui(egui::RawInput::default(), |_ui| {});
        ctx
    }

    /// สัญลักษณ์ ⌘ (U+2318) ต้องวาดออกมาได้จริง ไม่ใช่กล่องสี่เหลี่ยม "tofu"
    /// — เคยพลาดเพราะ Tahoma ไม่มีอักขระนี้ และ fallback ไป Segoe UI Symbol ไม่สำเร็จ
    #[test]
    fn font_has_command_symbol() {
        let ctx = ctx_with_app_fonts();

        let font_id = egui::FontId::new(12.0, egui::FontFamily::Proportional);
        assert!(
            ctx.fonts_mut(|f| f.has_glyph(&font_id, '\u{2318}')),
            "ฟอนต์ต้องมีสัญลักษณ์ ⌘ (U+2318) ไม่ใช่กล่อง tofu"
        );
    }

    /// อักษรไทยต้องวาดได้ครบ (ฟอนต์เริ่มต้นของ egui ไม่มีภาษาไทย)
    #[test]
    fn font_has_thai_glyphs() {
        let ctx = ctx_with_app_fonts();

        let font_id = egui::FontId::new(14.0, egui::FontFamily::Proportional);
        for ch in ['ส', 'ก', 'ไ', 'ท', 'ค', 'ำ'] {
            assert!(
                ctx.fonts_mut(|f| f.has_glyph(&font_id, ch)),
                "ฟอนต์ต้องมีอักษรไทย {ch}"
            );
        }
    }

    /// ต้องมีอย่างน้อย 2 ฟอนต์ในสายอักษร Proportional
    /// (ตัวหลักสำหรับไทย + ตัวสำรองสำหรับสัญลักษณ์) ไม่งั้น fallback จะไม่ทำงาน
    #[test]
    fn proportional_family_has_fallback_chain() {
        let fonts = build_font_definitions();
        let chain = fonts
            .families
            .get(&egui::FontFamily::Proportional)
            .expect("ต้องมีสายอักษร Proportional");
        assert!(
            chain.len() >= 2,
            "สายอักษร Proportional ต้องมีฟอนต์สำรอง (ได้ {} ฟอนต์)",
            chain.len()
        );
    }

    /// ค่าเป้าหมายการสลับภาษาแบบพิเศษต้องแสดงเป็นข้อความไทยที่อ่านเข้าใจได้
    #[test]
    fn layout_target_label_is_readable() {
        assert_eq!(layout_target_label("cycle"), "ตามรายการที่ตั้งไว้");
        assert_eq!(layout_target_label("next"), "ภาษาถัดไปที่ติดตั้ง");
        // ชื่อภาษาจริง (เช่น "en-US") ต้องแสดงตามเดิม
        assert_eq!(layout_target_label("en-US"), "en-US");
    }

    /// ระดับ log ต้องแสดงเป็นข้อความไทยที่อ่านเข้าใจได้
    #[test]
    fn log_level_label_is_readable() {
        for lv in ["error", "warn", "info", "debug", "trace"] {
            let label = log_level_label(lv);
            assert!(
                label.contains(lv),
                "ป้ายของระดับ {lv} ต้องบอกชื่อระดับด้วย (ได้ {label})"
            );
            assert!(
                label.len() > lv.len(),
                "ป้ายของระดับ {lv} ต้องมีคำอธิบายเพิ่ม (ได้ {label})"
            );
        }
    }

    /// คีย์ลัดที่ถูกต้อง → สถานะเขียว
    #[test]
    fn shortcut_status_accepts_valid_combo() {
        assert_eq!(
            shortcut_status("ctrl+shift+3"),
            ui::FieldStatus::Valid("คีย์ลัดถูกต้อง".to_string())
        );
        // ช่องว่างรอบ ๆ ต้องถูกตัดออกก่อนตรวจ
        assert_eq!(
            shortcut_status("  alt+space  "),
            ui::FieldStatus::Valid("คีย์ลัดถูกต้อง".to_string())
        );
    }

    /// คีย์ลัดที่ผิด → สถานะแดงพร้อมบอกเหตุผลเป็นภาษาไทย
    #[test]
    fn shortcut_status_explains_invalid_combo() {
        match shortcut_status("ctrl+ไม่มีปุ่ม") {
            ui::FieldStatus::Invalid(msg) => {
                assert!(
                    msg.contains("ไม่รู้จักปุ่ม"),
                    "ต้องบอกว่าไม่รู้จักปุ่มอะไร (ได้ {msg})"
                );
            }
            other => panic!("ต้องเป็น Invalid แต่ได้ {other:?}"),
        }

        // ไม่มี modifier → ต้องเตือนด้วย
        match shortcut_status("q") {
            ui::FieldStatus::Invalid(msg) => assert!(
                msg.contains("modifier"),
                "ต้องเตือนว่าต้องมีปุ่ม modifier (ได้ {msg})"
            ),
            other => panic!("ต้องเป็น Invalid แต่ได้ {other:?}"),
        }

        // ว่างเปล่า → ต้องเตือนด้วย
        assert!(matches!(
            shortcut_status("   "),
            ui::FieldStatus::Invalid(_)
        ));
    }
}
