use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::Duration;

use tracing_subscriber::EnvFilter;

/// ขนาดสูงสุดของไฟล์ log ก่อนจะหมุนไฟล์เก่าทิ้ง (2 MB)
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

/// จำนวนบรรทัดที่ยอมค้างในคิวก่อนทิ้ง (กันความจำบวมตอนดิสก์ค้าง)
const LOG_QUEUE_MAX: usize = 4096;

/// โฟลเดอร์เก็บ log: %LOCALAPPDATA%\MacKeys\logs
pub fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("MacKeys")
        .join("logs")
}

/// ไฟล์ log ที่ thread เขียน log เป็นเจ้าของ
///
/// การหมุนไฟล์ทำที่นี่ (ไม่ใช่ที่อื่น) เพราะบน Windows เปิดไฟล์ค้างไว้แล้ว
/// จะ `rename` ไม่ได้ — ต้องปิดก่อน แล้วค่อยเปิดใหม่
struct LogFile {
    path: PathBuf,
    file: Option<std::fs::File>,
}

impl LogFile {
    fn open(&mut self) {
        self.file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .ok();
    }

    fn write_line(&mut self, line: &str) {
        if let Some(f) = self.file.as_mut() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.write_all(b"\n");
        }
    }

    fn flush(&mut self) {
        if let Some(f) = self.file.as_mut() {
            let _ = f.flush();
        }
    }

    /// หมุนไฟล์เมื่อใหญ่เกินกำหนด
    fn maybe_rotate(&mut self) {
        let Ok(meta) = std::fs::metadata(&self.path) else {
            return;
        };
        if meta.len() <= MAX_LOG_BYTES {
            return;
        }
        // ต้องปิดไฟล์ก่อน ถึงจะ rename ได้บน Windows
        self.file = None;
        let old = self.path.with_extension("log.1");
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::rename(&self.path, &old);
        self.open();
    }
}

/// writer ที่ส่งบรรทัด log ไปยัง thread เขียนไฟล์ (ไม่เขียนไฟล์เอง!)
///
/// สำคัญมาก: ถ้าเขียนไฟล์ตรง ๆ ใน thread ที่ติดตั้ง hook ตอนดิสก์ค้าง
/// → thread นั้นค้าง → hook ตายเงียบ → ผู้ใช้กดอะไรก็ไม่มีผล
/// (อาการ: "กดไม่เปลี่ยน" แบบทั้งช่วง ไม่ใช่แค่กดครั้งเดียว)
struct ChannelWriter {
    tx: SyncSender<String>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf).into_owned();
        // ห้าม block เด็ดขาด — ถ้าคิวเต็มก็ทิ้ง log ทิ้ง (log ไม่สำคัญพอจะไปหยุด input path)
        match self.tx.try_send(text) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for ChannelWriter {
    type Writer = ChannelWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ChannelWriter { tx: self.tx.clone() }
    }
}

/// ตั้งค่า logger ลงไฟล์ (ไม่มี console เพื่อไม่รบกวนตอนรันเป็น tray app)
///
/// สถาปัตยกรรม: ทุกอย่างที่ `tracing` บันทึกจะถูกส่งผ่านคิวไปยัง **thread แยก**
/// เป็นคนเขียนไฟล์จริง → thread หลัก (ที่ติดตั้ง hook) ไม่เคยแตะดิสก์
pub fn init(level: &str) {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("mackeys={level},warn")));

    // จับ panic ให้กลายเป็นข้อความใน log แทนที่จะทำให้โปรแกรมตายเงียบ ๆ
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        prev_hook(info);
    }));

    let path = dir.join("mackeys.log");

    // thread เขียน log: รับบรรทัดจากคิว → เขียนลงไฟล์ → หมุนไฟล์เมื่อใหญ่เกินกำหนด
    let (tx, rx) = std::sync::mpsc::sync_channel::<String>(LOG_QUEUE_MAX);
    std::thread::Builder::new()
        .name("mackeys-log".into())
        .spawn(move || {
            let mut lf = LogFile {
                path: path.clone(),
                file: None,
            };
            lf.open();
            let mut buf = String::new();
            loop {
                match rx.recv_timeout(Duration::from_millis(150)) {
                    Ok(chunk) => {
                        buf.push_str(&chunk);
                        while let Some(idx) = buf.find('\n') {
                            let line = buf[..idx].to_string();
                            buf.drain(..=idx);
                            lf.write_line(&line);
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // ไม่มีอะไรใหม่ → เขียนที่ค้าง + flush + เช็คหมุนไฟล์
                        if !buf.is_empty() {
                            let line = std::mem::take(&mut buf);
                            lf.write_line(&line);
                        }
                        lf.flush();
                        lf.maybe_rotate();
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            lf.flush();
        })
        .ok();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_target(false)
        .with_writer(ChannelWriter { tx })
        .try_init();
}
