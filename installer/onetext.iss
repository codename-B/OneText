; OneText Installer Script for Inno Setup 6.6
; Builds a Windows installer with file associations

#define MyAppName "OneText"
#define MyAppVersion "0.1.4"
#define MyAppPublisher "OneText"
#define MyAppURL "https://github.com/codename-B/OneText"
#define MyAppExeName "onetext.exe"

[Setup]
; Unique app ID - generated GUID
AppId={{A1B2C3D4-E5F6-7890-ABCD-EF1234567890}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
; Show license during install
LicenseFile=..\assets\License.txt
; Output settings
OutputDir=Output
OutputBaseFilename=OneText-Setup-{#MyAppVersion}
; Installer appearance
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
; Compression
Compression=lzma2
SolidCompression=yes
; Modern look
WizardStyle=modern
; Require admin for Program Files installation
PrivilegesRequired=admin
; Register file associations
ChangesAssociations=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "txtassoc"; Description: "Associate with .txt files"; GroupDescription: "File associations:"; Flags: unchecked
Name: "mdassoc"; Description: "Associate with .md files"; GroupDescription: "File associations:"; Flags: unchecked
Name: "logassoc"; Description: "Associate with .log files"; GroupDescription: "File associations:"; Flags: unchecked

[Files]
; Main executable
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; Assets folder (themes, fonts, license)
Source: "..\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs
; Exclude source-only files if any
; Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start Menu
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
; Desktop (optional)
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; .txt file association
Root: HKA; Subkey: "Software\Classes\.txt\OpenWithProgids"; ValueType: string; ValueName: "OneText.txt"; ValueData: ""; Flags: uninsdeletevalue; Tasks: txtassoc
Root: HKA; Subkey: "Software\Classes\OneText.txt"; ValueType: string; ValueName: ""; ValueData: "Text Document"; Flags: uninsdeletekey; Tasks: txtassoc
Root: HKA; Subkey: "Software\Classes\OneText.txt\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Tasks: txtassoc
Root: HKA; Subkey: "Software\Classes\OneText.txt\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: txtassoc

; .md file association
Root: HKA; Subkey: "Software\Classes\.md\OpenWithProgids"; ValueType: string; ValueName: "OneText.md"; ValueData: ""; Flags: uninsdeletevalue; Tasks: mdassoc
Root: HKA; Subkey: "Software\Classes\OneText.md"; ValueType: string; ValueName: ""; ValueData: "Markdown Document"; Flags: uninsdeletekey; Tasks: mdassoc
Root: HKA; Subkey: "Software\Classes\OneText.md\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Tasks: mdassoc
Root: HKA; Subkey: "Software\Classes\OneText.md\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: mdassoc

; .log file association
Root: HKA; Subkey: "Software\Classes\.log\OpenWithProgids"; ValueType: string; ValueName: "OneText.log"; ValueData: ""; Flags: uninsdeletevalue; Tasks: logassoc
Root: HKA; Subkey: "Software\Classes\OneText.log"; ValueType: string; ValueName: ""; ValueData: "Log File"; Flags: uninsdeletekey; Tasks: logassoc
Root: HKA; Subkey: "Software\Classes\OneText.log\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Tasks: logassoc
Root: HKA; Subkey: "Software\Classes\OneText.log\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: logassoc

; Add to "Open with" menu for all text files
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#MyAppName}"
Root: HKA; Subkey: "Software\Classes\Applications\{#MyAppExeName}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" ""%1"""

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
