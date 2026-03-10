# WiX Installer Files

This directory contains files needed to build the Windows MSI installer.

## Required Files

### a2rs.ico
Place your application icon here (256x256 recommended, multi-resolution ICO format).

You can create one from a PNG using:
- ImageMagick: `convert icon.png -define icon:auto-resize=256,128,64,48,32,16 a2rs.ico`
- Online tools: https://convertico.com/

If you don't have an icon, create a placeholder:
```powershell
# Create a simple placeholder (requires .NET)
Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap(256,256)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::DarkGreen)
$font = New-Object System.Drawing.Font("Arial", 72)
$g.DrawString("A2", $font, [System.Drawing.Brushes]::White, 50, 70)
$bmp.Save("a2rs.ico", [System.Drawing.Imaging.ImageFormat]::Icon)
```

## Building the MSI

1. Install WiX Toolset: https://wixtoolset.org/releases/
2. Install cargo-wix: `cargo install cargo-wix`
3. Build the MSI: `cargo wix`

The MSI will be created in `target/wix/`

## Customization

Edit `main.wxs` to customize:
- Product name and version
- Installation directory
- Shortcuts
- Features
