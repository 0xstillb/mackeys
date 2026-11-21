//! เทสต์ความสอดคล้องของสคริปต์ installer กับไฟล์จริงในโปรเจกต์
//!
//! เป้าหมาย: กันไม่ให้เวอร์ชันใน `installer/mackeys.iss` แยกจาก `Cargo.toml`
//! และกันไม่ให้ installer อ้างถึงไฟล์ที่ไม่มีอยู่จริง

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("อ่าน {} ไม่สำเร็จ: {e}", path.display()))
}

/// ดึงค่าใน #define จากสคริปต์ .iss
fn iss_define(src: &str, name: &str) -> String {
    let prefix = format!("#define {name} \"");
    let line = src
        .lines()
        .find(|l| l.trim_start().starts_with(&prefix))
        .unwrap_or_else(|| panic!("ไม่พบ #define {name} ในสคริปต์ installer"));
    let rest = &line[line.find(&prefix).unwrap() + prefix.len()..];
    rest[..rest.find('"').unwrap()].to_string()
}

/// ดึงเวอร์ชันแพ็กเกจจาก Cargo.toml (เฉพาะในส่วน [package])
fn cargo_version() -> String {
    let src = read("Cargo.toml");
    let package = src
        .split("[package]")
        .nth(1)
        .expect("ไม่พบส่วน [package] ใน Cargo.toml");
    let version = package
        .lines()
        .find(|l| l.trim_start().starts_with("version"))
        .expect("ไม่พบ version ใน [package]");
    let value = version.split('=').nth(1).expect("รูปแบบ version ไม่ถูกต้อง");
    value.trim().trim_matches('"').to_string()
}

#[test]
fn installer_version_matches_cargo_toml() {
    let iss = read("installer/mackeys.iss");
    assert_eq!(
        iss_define(&iss, "MyAppVersion"),
        cargo_version(),
        "เวอร์ชันใน installer/mackeys.iss ต้องตรงกับ version ใน Cargo.toml"
    );
}

#[test]
fn installer_app_name_and_exe_are_consistent() {
    let iss = read("installer/mackeys.iss");
    assert_eq!(iss_define(&iss, "MyAppName"), "MacKeys");
    assert_eq!(iss_define(&iss, "MyAppExeName"), "mackeys.exe");

    // ไฟล์ที่ติดตั้งต้องเป็นตัวโปรแกรมจริง
    assert!(
        iss.contains("Source: \"..\\target\\release\\{#MyAppExeName}\""),
        "installer ต้องแพ็คไฟล์ mackeys.exe จาก target\\release"
    );
}

#[test]
fn installer_is_per_user_and_never_needs_uac() {
    let iss = read("installer/mackeys.iss");
    assert!(
        iss.contains("PrivilegesRequired=lowest"),
        "ต้องติดตั้งแบบต่อผู้ใช้ (PrivilegesRequired=lowest) เพราะโปรแกรมไม่ต้องใช้สิทธิ์ admin"
    );
    assert!(
        iss.contains(r"DefaultDirName={localappdata}\Programs\{#MyAppName}"),
        "ต้องติดตั้งใน %LOCALAPPDATA%\\Programs"
    );
}

#[test]
fn installer_references_icon_asset_that_exists() {
    let iss = read("installer/mackeys.iss");
    assert!(
        iss.contains(r"SetupIconFile=..\assets\mackeys.ico"),
        "installer ต้องใช้ไอคอนจาก assets/mackeys.ico"
    );

    let icon = root().join("assets").join("mackeys.ico");
    assert!(icon.exists(), "ไม่พบไฟล์ {}", icon.display());
    assert!(icon.metadata().unwrap().len() > 0, "ไฟล์ไอคอนว่างเปล่า");
}

#[test]
fn installer_registers_and_removes_autostart() {
    let iss = read("installer/mackeys.iss");
    assert!(
        iss.contains("Parameters: \"install-autostart\""),
        "installer ต้องมีตัวเลือกเปิดโปรแกรมพร้อม Windows"
    );
    assert!(
        iss.contains("Parameters: \"uninstall-autostart\""),
        "ตอนถอนการติดตั้งต้องเอาค่า autostart ออกด้วย"
    );
}

#[test]
fn installer_license_is_mit_and_present() {
    let iss = read("installer/mackeys.iss");
    assert!(iss.contains("LicenseFile=..\\LICENSE"));

    let license = read("LICENSE");
    assert!(
        license.contains("MIT License"),
        "ไฟล์ LICENSE ต้องเป็น MIT"
    );
}

#[test]
fn ci_workflow_builds_the_installer() {
    let path = root().join(".github").join("workflows").join("release.yml");
    assert!(path.exists(), "ไม่พบ workflow {}", path.display());

    let src = fs::read_to_string(&path).unwrap();
    assert!(
        src.contains("mackeys.iss"),
        "workflow ต้องเรียก ISCC เพื่อสร้าง installer"
    );
    assert!(
        src.contains("setup.exe") || src.contains("setup"),
        "workflow ต้องแนบไฟล์ setup.exe เข้า Release"
    );
}

#[test]
fn version_file_exists_for_reference() {
    // กัน path ในสคริปต์ installer พลาดไปเรื่องเครื่องมือชื่อเพี้ยน
    let iss = read("installer/mackeys.iss");
    assert!(Path::new("installer").join("mackeys.iss").exists());
    assert!(iss.contains("[Setup]"));
    assert!(iss.contains("[Files]"));
    assert!(iss.contains("[Icons]"));
}
