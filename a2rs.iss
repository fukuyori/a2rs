; A2RS installer script for Inno Setup 6
#define MyAppName "A2RS"
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#ifndef MyAppSourceDir
  #define MyAppSourceDir "."
#endif
#ifndef MyOutputDir
  #define MyOutputDir "target\inno"
#endif

[Setup]
AppId={{8DCC9E4A-7E3B-4E9E-A7A2-6C0D2B9A1001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
DefaultDirName={autopf}\A2RS
DefaultGroupName=A2RS
UninstallDisplayIcon={app}\a2rs.exe
OutputDir={#MyOutputDir}
OutputBaseFilename=a2rs-{#MyAppVersion}-windows-x64-setup
SetupIconFile=installer_assets\a2rs.ico
LicenseFile=installer_assets\LICENSE.txt
WizardImageFile=installer_assets\WixUIDialogBmp.png
WizardSmallImageFile=installer_assets\WixUIBannerBmp.png
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Dirs]
Name: "{app}\roms"
Name: "{app}\disks"
Name: "{app}\docs"

[Files]
Source: "{#MyAppSourceDir}\a2rs.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#MyAppSourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#MyAppSourceDir}\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "{#MyAppSourceDir}\roms\*"; DestDir: "{app}\roms"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist
Source: "{#MyAppSourceDir}\disks\*"; DestDir: "{app}\disks"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist
Source: "{#MyAppSourceDir}\docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Icons]
Name: "{group}\A2RS"; Filename: "{app}\a2rs.exe"
Name: "{group}\Uninstall A2RS"; Filename: "{uninstallexe}"
Name: "{autodesktop}\A2RS"; Filename: "{app}\a2rs.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop icon"; Flags: unchecked

[Run]
Filename: "{app}\a2rs.exe"; Description: "Launch A2RS"; Flags: nowait postinstall skipifsilent
