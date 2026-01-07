@echo off
REM Windows installer builder for A2RS
REM Run from project root directory

setlocal enabledelayedexpansion

echo === A2RS Installer Builder (Windows) ===
echo.

REM Get version from Cargo.toml
for /f "tokens=3 delims= " %%a in ('findstr /r "^version" Cargo.toml') do (
    set VERSION=%%a
    set VERSION=!VERSION:"=!
    goto :version_found
)
:version_found
echo Version: %VERSION%

REM Build release binary
echo.
echo Building release binary...
cargo build --release --features full
if errorlevel 1 (
    echo Build failed!
    exit /b 1
)

REM Create distribution directory
set DIST_DIR=target\dist\a2rs-%VERSION%-windows-x64
echo.
echo Creating distribution: %DIST_DIR%

if exist "%DIST_DIR%" rmdir /s /q "%DIST_DIR%"
mkdir "%DIST_DIR%"
mkdir "%DIST_DIR%\roms"
mkdir "%DIST_DIR%\disks"
mkdir "%DIST_DIR%\saves"
mkdir "%DIST_DIR%\screenshots"

REM Copy files
copy target\release\a2rs.exe "%DIST_DIR%\"
copy README.md "%DIST_DIR%\" 2>nul

REM Create default config
(
echo {
echo   "speed": 1,
echo   "fast_disk": true,
echo   "sound_enabled": true,
echo   "quality_level": 4,
echo   "auto_quality": true,
echo   "window_width": 800,
echo   "window_height": 600,
echo   "current_slot": 0,
echo   "rom_dir": "roms",
echo   "disk_dir": "disks",
echo   "screenshot_dir": "screenshots",
echo   "save_dir": "saves"
echo }
) > "%DIST_DIR%\apple2_config.json"

REM Create ZIP
echo.
echo Creating ZIP archive...
cd target\dist
if exist "a2rs-%VERSION%-windows-x64.zip" del "a2rs-%VERSION%-windows-x64.zip"

REM Try PowerShell compression first
powershell -Command "Compress-Archive -Path 'a2rs-%VERSION%-windows-x64' -DestinationPath 'a2rs-%VERSION%-windows-x64.zip'" 2>nul
if errorlevel 1 (
    REM Try 7-Zip
    where 7z >nul 2>nul
    if not errorlevel 1 (
        7z a -tzip "a2rs-%VERSION%-windows-x64.zip" "a2rs-%VERSION%-windows-x64"
    ) else (
        echo Warning: Could not create ZIP. Install 7-Zip or use PowerShell 5+
    )
)
cd ..\..

echo.
echo Portable ZIP created: target\dist\a2rs-%VERSION%-windows-x64.zip

REM Check for cargo-wix
echo.
cargo install --list | findstr /c:"cargo-wix" >nul
if not errorlevel 1 (
    echo Building MSI installer...
    if not exist wix\main.wxs (
        echo Initializing WiX...
        cargo wix init
    )
    cargo wix
    echo MSI installer created in target\wix\
) else (
    echo.
    echo To create MSI installer:
    echo   cargo install cargo-wix
    echo   cargo wix init
    echo   cargo wix
)

echo.
echo === Build complete ===
echo Output files:
dir /b target\dist\*.zip 2>nul
dir /b target\wix\*.msi 2>nul

endlocal
