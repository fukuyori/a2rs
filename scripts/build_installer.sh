#!/bin/bash
# Cross-platform installer builder for A2RS
# Detects the current OS and builds the appropriate installer

set -e

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Detect OS
case "$(uname -s)" in
    Linux*)
        OS="linux"
        ;;
    Darwin*)
        OS="macos"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        OS="windows"
        ;;
    *)
        echo "Unsupported OS: $(uname -s)"
        exit 1
        ;;
esac

echo "=== A2RS Installer Builder ==="
echo "Detected OS: $OS"
echo "Project directory: $PROJECT_DIR"
echo ""

# Build release binary first
echo "Building release binary..."
cargo build --release

case "$OS" in
    linux)
        echo ""
        echo "=== Building Linux packages ==="
        
        # Check for cargo-deb
        if command -v cargo-deb &> /dev/null || cargo install --list | grep -q "cargo-deb"; then
            echo "Building .deb package..."
            cargo deb
            echo "DEB package created in target/debian/"
        else
            echo "cargo-deb not found. Install with: cargo install cargo-deb"
        fi
        
        # Create portable tarball
        echo ""
        echo "Creating portable tarball..."
        VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
        DIST_DIR="target/dist/a2rs-${VERSION}-linux-x86_64"
        
        mkdir -p "$DIST_DIR"/{roms,disks,saves,screenshots}
        cp target/release/a2rs "$DIST_DIR/"
        cp README.md "$DIST_DIR/" 2>/dev/null || true
        
        # Create default config
        cat > "$DIST_DIR/apple2_config.json" << 'EOF'
{
  "speed": 1,
  "fast_disk": true,
  "sound_enabled": true,
  "quality_level": 4,
  "auto_quality": true,
  "window_width": 800,
  "window_height": 600,
  "current_slot": 0,
  "rom_dir": "roms",
  "disk_dir": "disks",
  "screenshot_dir": "screenshots",
  "save_dir": "saves"
}
EOF

        # Create launcher script
        cat > "$DIST_DIR/a2rs.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
./a2rs "$@"
EOF
        chmod +x "$DIST_DIR/a2rs.sh"
        
        # Create tarball
        cd target/dist
        tar -czvf "a2rs-${VERSION}-linux-x86_64.tar.gz" "a2rs-${VERSION}-linux-x86_64"
        cd "$PROJECT_DIR"
        
        echo "Portable tarball created: target/dist/a2rs-${VERSION}-linux-x86_64.tar.gz"
        
        # AppImage (if available)
        if command -v linuxdeploy &> /dev/null; then
            echo ""
            echo "Building AppImage..."
            # AppImage build would go here
        fi
        ;;
        
    macos)
        echo ""
        echo "=== Building macOS package ==="
        
        VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
        APP_NAME="A2RS"
        BUNDLE_DIR="target/dist/${APP_NAME}.app"
        
        # Create .app bundle structure
        mkdir -p "$BUNDLE_DIR/Contents/MacOS"
        mkdir -p "$BUNDLE_DIR/Contents/Resources"/{roms,disks,saves,screenshots}
        
        # Copy binary
        cp target/release/a2rs "$BUNDLE_DIR/Contents/MacOS/"
        
        # Create Info.plist
        cat > "$BUNDLE_DIR/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>A2RS Apple II Emulator</string>
    <key>CFBundleIdentifier</key>
    <string>com.a2rs.emulator</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>a2rs</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Apple II Disk Image</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>dsk</string>
                <string>do</string>
                <string>po</string>
                <string>nib</string>
            </array>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
        </dict>
    </array>
</dict>
</plist>
EOF

        # Create default config
        cat > "$BUNDLE_DIR/Contents/Resources/apple2_config.json" << 'EOF'
{
  "speed": 1,
  "fast_disk": true,
  "sound_enabled": true,
  "quality_level": 4,
  "auto_quality": true,
  "window_width": 800,
  "window_height": 600,
  "current_slot": 0,
  "rom_dir": "roms",
  "disk_dir": "disks",
  "screenshot_dir": "screenshots",
  "save_dir": "saves"
}
EOF
        
        echo ".app bundle created: $BUNDLE_DIR"
        
        # Create DMG if hdiutil is available
        if command -v hdiutil &> /dev/null; then
            echo ""
            echo "Creating DMG..."
            DMG_DIR="target/dist/dmg"
            mkdir -p "$DMG_DIR"
            cp -r "$BUNDLE_DIR" "$DMG_DIR/"
            
            # Create Applications symlink
            ln -sf /Applications "$DMG_DIR/Applications"
            
            hdiutil create -volname "A2RS" -srcfolder "$DMG_DIR" \
                -ov -format UDZO "target/dist/A2RS-${VERSION}.dmg"
            
            rm -rf "$DMG_DIR"
            echo "DMG created: target/dist/A2RS-${VERSION}.dmg"
        fi
        ;;
        
    windows)
        echo ""
        echo "=== Building Windows installer ==="
        
        VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
        
        # Create portable ZIP
        echo "Creating portable ZIP..."
        DIST_DIR="target/dist/a2rs-${VERSION}-windows-x64"
        
        mkdir -p "$DIST_DIR"/{roms,disks,saves,screenshots}
        cp target/release/a2rs.exe "$DIST_DIR/"
        cp README.md "$DIST_DIR/" 2>/dev/null || true
        
        # Create default config
        cat > "$DIST_DIR/apple2_config.json" << 'EOF'
{
  "speed": 1,
  "fast_disk": true,
  "sound_enabled": true,
  "quality_level": 4,
  "auto_quality": true,
  "window_width": 800,
  "window_height": 600,
  "current_slot": 0,
  "rom_dir": "roms",
  "disk_dir": "disks",
  "screenshot_dir": "screenshots",
  "save_dir": "saves"
}
EOF
        
        # Create ZIP
        cd target/dist
        if command -v 7z &> /dev/null; then
            7z a -tzip "a2rs-${VERSION}-windows-x64.zip" "a2rs-${VERSION}-windows-x64"
        elif command -v zip &> /dev/null; then
            zip -r "a2rs-${VERSION}-windows-x64.zip" "a2rs-${VERSION}-windows-x64"
        fi
        cd "$PROJECT_DIR"
        
        echo "Portable ZIP created: target/dist/a2rs-${VERSION}-windows-x64.zip"
        
        # WiX MSI installer (if cargo-wix is installed)
        if cargo install --list | grep -q "cargo-wix"; then
            echo ""
            echo "Building MSI installer..."
            cargo wix
            echo "MSI installer created in target/wix/"
        else
            echo ""
            echo "To create MSI installer, install cargo-wix:"
            echo "  cargo install cargo-wix"
            echo "  cargo wix init"
            echo "  cargo wix"
        fi
        ;;
esac

echo ""
echo "=== Build complete ==="
echo "Output files in: target/dist/"
ls -la target/dist/ 2>/dev/null || true
