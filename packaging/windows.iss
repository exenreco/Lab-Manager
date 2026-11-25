#ifndef AppVersion
  #define AppVersion "0.1.0-alpha.1"
#endif
[Setup]
AppId={{24890BC1-15A7-46AD-9EC2-FD591EF64316}
AppName=Lab Manager
AppVersion={#AppVersion}
AppPublisher=Exenreco Bell
AppPublisherURL=https://github.com/exenreco/Lab-Manager
DefaultDirName={localappdata}\Programs\Lab Manager
DefaultGroupName=Lab Manager
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=Lab-Manager-{#AppVersion}-windows-x64-setup
LicenseFile=TERMS.txt
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\lab-manager.exe
CloseApplications=yes
RestartApplications=no
SignTool=release
SignedUninstaller=yes
[Tasks]
Name: "startup"; Description: "Run Lab Manager when I sign in to Windows"; Flags: unchecked
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked
[Files]
Source: "..\target\release\lab-manager.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion
Source: "TERMS.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\icons\*-LICENSE"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\dist\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion
[Icons]
Name: "{group}\Lab Manager"; Filename: "{app}\lab-manager.exe"
Name: "{autodesktop}\Lab Manager"; Filename: "{app}\lab-manager.exe"; Tasks: desktopicon
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "LabManager"; ValueData: """{app}\lab-manager.exe"""; Flags: uninsdeletevalue; Tasks: startup
[Run]
Filename: "{app}\lab-manager.exe"; Description: "Launch Lab Manager"; Flags: nowait postinstall skipifsilent
[Code]
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and not WizardIsTaskSelected('startup') then
    RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'LabManager');
end;
