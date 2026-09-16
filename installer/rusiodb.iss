; Script de Inno Setup para RusioDB.
; La versión se sustituye desde la variable de entorno RUSIODB_VERSION, que
; pone el workflow de CI (.github/workflows/release.yml) a partir del tag
; empujado (vX.Y.Z) — no se edita a mano por release.
;
; Instalación por usuario (sin admin/UAC): esto es lo que permite que el
; autoupdater de la app reemplace el .exe más adelante sin pedir elevación.
;
; Compilar localmente (necesita Inno Setup 6 instalado):
;   iscc installer\rusiodb.iss
; El instalador resultante queda en installer\Output\.

#define MyAppName "RusioDB"
#define MyAppVersion GetEnv("RUSIODB_VERSION")
#define MyAppPublisher "Rodrigo Guerrero"
#define MyAppURL "https://github.com/Fiambre/RusioDB"
#define MyAppExeName "rusiodb.exe"

[Setup]
AppId={{8B6F0B1A-5C7E-4C1F-9A3D-2E7B6F0C1D9A}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\RusioDB
DefaultGroupName=RusioDB
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
OutputDir=Output
OutputBaseFilename=RusioDBSetup-x86_64-pc-windows-msvc
SetupIconFile=..\assets\icon.ico
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "Crear un acceso directo en el escritorio"; Flags: unchecked

[Files]
Source: "..\target\release\rusiodb.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\RusioDB"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Desinstalar RusioDB"; Filename: "{uninstallexe}"
Name: "{userdesktop}\RusioDB"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Ejecutar RusioDB"; Flags: nowait postinstall skipifsilent
