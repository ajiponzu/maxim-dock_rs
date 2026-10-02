; MaXImDock v2 per-user Windows installer.
#define AppName "MaXImDock"
#define AppVersion "0.1.0"
#define AppPublisher "MaXImDock"
#define AppExeName "MaXImDock-v2-x86_64.exe"

[Setup]
AppId={{D5A2B4AC-91D6-48A6-B160-81B1BA20B4DD}
AppName={#AppName} v2
AppVersion={#AppVersion}
AppVerName={#AppName} v2
AppPublisher={#AppPublisher}
DefaultDirName={localappdata}\Programs\MaXImDock
DefaultGroupName=MaXImDock
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=MaXImDock-v2-Setup-x64
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
LanguageDetectionMethod=locale
VersionInfoVersion=0.1.0.0
VersionInfoProductName=MaXImDock v2
VersionInfoProductVersion=0.1.0
CloseApplications=yes
RestartApplications=no
Uninstallable=yes

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "デスクトップにショートカットを作成する"; GroupDescription: "追加のショートカット:"; Flags: unchecked

[Files]
Source: "..\dist\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\MaXImDock v2"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\MaXImDock v2"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "MaXImDock v2 を起動"; Flags: nowait postinstall skipifsilent
