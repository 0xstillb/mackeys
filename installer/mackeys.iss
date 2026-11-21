; MacKeys — Inno Setup script (ต่อผู้ใช้ ไม่ต้องขึ้น UAC)
;
; สร้างไฟล์ setup.exe ด้วย:
;   ISCC.exe installer\mackeys.iss
; ต้องมี target\release\mackeys.exe และ assets\mackeys.ico อยู่ก่อน

#define MyAppName "MacKeys"
#define MyAppVersion "0.2.0"
#define MyAppPublisher "0xstillb"
#define MyAppURL "https://github.com/0xstillb/mackeys"
#define MyAppExeName "mackeys.exe"

[Setup]
AppId={{A7E3C1F2-9B44-4D8E-B1A6-2F5C7E93D410}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
SetupIconFile=..\assets\mackeys.ico
OutputDir=..\dist
OutputBaseFilename=MacKeys-{#MyAppVersion}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\LICENSE
SetupLogging=yes
MinVersion=10.0
; ต่อผู้ใช้ — ติดตั้งใน %LOCALAPPDATA%\Programs ไม่ต้องขึ้น UAC
; (ถ้าผู้ใช้เปิดด้วยสิทธิ์ admin จะถามว่าจะติดตั้งให้ทั้งเครื่องหรือเฉพาะตัวเอง)
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription={#MyAppName} — คีย์ลัดสไตล์ macOS สำหรับ Windows
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}

[Languages]
Name: "thai"; MessagesFile: "compiler:Languages\Thai.isl"

[Tasks]
Name: "autostart"; Description: "ให้ MacKeys เปิดอัตโนมัติเมื่อเปิด Windows"; GroupDescription: "ตัวเลือกการเริ่มทำงาน"; Flags: checkedonce
Name: "desktopicon"; Description: "สร้างไอคอนบนเดสก์ท็อป"; GroupDescription: "ทางลัดเพิ่มเติม"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\ถอนการติดตั้ง MacKeys"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; ตั้งค่าให้รันพร้อม Windows (เขียนค่าใน HKCU\...\Run)
Filename: "{app}\{#MyAppExeName}"; Parameters: "install-autostart"; Tasks: autostart; Flags: runhidden
Filename: "{app}\{#MyAppExeName}"; Description: "เปิด MacKeys"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; ปิดโปรแกรมก่อนถอนการติดตั้ง (ไฟล์ exe จะถูกล็อกถ้ายังรันอยู่)
Filename: "taskkill.exe"; Parameters: "/IM {#MyAppExeName} /F"; Flags: runhidden; RunOnceId: "StopMacKeys"
; เอาค่า autostart ออกจาก HKCU\...\Run
Filename: "{app}\{#MyAppExeName}"; Parameters: "uninstall-autostart"; Flags: runhidden; RunOnceId: "RemoveAutostart"
