; Inno Setup installer script for QN Bench (Windows x64)
; Installs per-user under %LOCALAPPDATA%\Programs\QN Bench so no UAC admin prompt
; is required and the in-app self-updater can update cleanly in place.

#ifndef AppVersion
  #define AppVersion "1.0.3"
#endif

[Setup]
AppId={{8F6C6C14-4C93-4D4B-9D8A-1550A9B7E301}
AppName=QN Bench
AppVersion={#AppVersion}
AppVerName=QN Bench {#AppVersion}
AppPublisher=Al Anderson
AppPublisherURL=https://skcalanderson2.github.io/qn_bench/
AppSupportURL=https://github.com/skcalanderson2/qn_bench/issues
AppUpdatesURL=https://github.com/skcalanderson2/qn_bench/releases
DefaultDirName={localappdata}\Programs\QN Bench
DefaultGroupName=QN Bench
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=dist
OutputBaseFilename=QN-Bench-{#AppVersion}-windows-x64-setup
SetupIconFile=build\QN-Bench.ico
UninstallDisplayIcon={app}\QN Bench.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "dist\QN Bench\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\QN Bench"; Filename: "{app}\QN Bench.exe"
Name: "{autodesktop}\QN Bench"; Filename: "{app}\QN Bench.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\QN Bench.exe"; Description: "{cm:LaunchProgram,QN Bench}"; Flags: nowait postinstall skipifsilent
