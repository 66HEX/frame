#define AppId "{{7D37707F-5F27-47B6-82EE-207B922EC013}"
#ifndef AppName
#define AppName "Frame"
#endif
#ifndef AppSetupName
#define AppSetupName "Frame-x86_64"
#endif
#ifndef AppVersion
#define AppVersion "0.31.0"
#endif
#ifndef OutputDir
#define OutputDir "..\target"
#endif
#ifndef ResourcesDir
#define ResourcesDir "..\target\inno\x86_64"
#endif

[Setup]
AppId={#AppId}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=66HEX
AppPublisherURL=https://github.com/66HEX/frame
AppSupportURL=https://github.com/66HEX/frame/issues
AppUpdatesURL=https://github.com/66HEX/frame/releases
DefaultDirName={localappdata}\Programs\Frame
DefaultGroupName=Frame
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename={#AppSetupName}
SetupIconFile={#ResourcesDir}\app-icon.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\Frame.exe
#ifdef SignToolName
SignTool={#SignToolName}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#ResourcesDir}\Frame.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#ResourcesDir}\frame-update-helper.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#ResourcesDir}\*.dll"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#ResourcesDir}\binaries\*"; DestDir: "{app}\binaries"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Frame"; Filename: "{app}\Frame.exe"
Name: "{autodesktop}\Frame"; Filename: "{app}\Frame.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Frame.exe"; Description: "{cm:LaunchProgram,Frame}"; Flags: nowait postinstall skipifsilent
