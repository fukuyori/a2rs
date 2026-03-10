# A2RS installer packaging set

Files:
- build_installer_complete.bat
- a2rs.iss
- wix/main.wxs
- installer_assets/a2rs.ico
- installer_assets/LICENSE.rtf
- installer_assets/LICENSE.txt
- installer_assets/WixUIBannerBmp.png
- installer_assets/WixUIDialogBmp.png

Usage:
1. Copy these files into the A2RS project root.
2. Ensure Inno Setup's ISCC.exe is in PATH for EXE installer generation.
3. Ensure WiX v6's wix.exe is in PATH for MSI generation.
4. Run build_installer_complete.bat from the project root.

Outputs:
- target/dist/a2rs-<version>-windows-x64.zip
- target/inno/a2rs-<version>-windows-x64-setup.exe
- target/wix/a2rs-<version>-windows-x64.msi
