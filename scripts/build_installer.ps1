param(
    [string]$Profile = "release",
    [string]$FeatureSet = "full",
    [switch]$SkipBuild,
    [switch]$SkipZip,
    [switch]$SkipInno,
    [switch]$SkipMsi
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$scriptDir = $PSScriptRoot
$root = Split-Path -Parent $scriptDir

function Write-Section([string]$Text) {
    Write-Host ""
    Write-Host "=== $Text ==="
}

function Get-CargoVersion {
    param([string]$ProjectRoot)

    $cargoToml = Join-Path $ProjectRoot "Cargo.toml"
    if (-not (Test-Path $cargoToml)) {
        throw "Cargo.toml not found: $cargoToml"
    }

    $content = Get-Content $cargoToml -Raw -Encoding UTF8

    $packageBlock = [regex]::Match(
        $content,
        '(?ms)^\[package\]\s*(.*?)^(?=\[|\z)'
    )

    if (-not $packageBlock.Success) {
        throw "[package] section was not found in Cargo.toml"
    }

    $versionMatch = [regex]::Match(
        $packageBlock.Groups[1].Value,
        '(?m)^version\s*=\s*"([^"]+)"'
    )

    if (-not $versionMatch.Success) {
        throw "version field was not found in [package] section"
    }

    return $versionMatch.Groups[1].Value
}

function Find-Tool([string[]]$Candidates) {
    foreach ($name in $Candidates) {
        $cmd = Get-Command $name -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Source }
    }
    return $null
}

function Ensure-Directory([string]$Path) {
    if (-not (Test-Path $Path)) {
        New-Item -ItemType Directory -Path $Path | Out-Null
    }
}

function Test-WixUiExtensionInstalled([string]$WixExe) {
    try {
        $output = & $WixExe extension list 2>$null
        if ($LASTEXITCODE -ne 0) { return $false }
        return ($output -match 'WixToolset\.UI\.wixext')
    }
    catch {
        return $false
    }
}

Write-Section "A2RS Installer Builder (Windows / PowerShell)"

$version = Get-CargoVersion -ProjectRoot $root
Write-Host "Version: $version"

$exeName = "a2rs.exe"
$targetDir = Join-Path $root "target"
$releaseDir = Join-Path $targetDir $Profile
$exePath = Join-Path $releaseDir $exeName
$distRoot = Join-Path $targetDir "dist"
$packageName = "a2rs-$version-windows-x64"
$packageDir = Join-Path $distRoot $packageName
$zipPath = Join-Path $distRoot "$packageName.zip"
$innoOutDir = Join-Path $targetDir "inno"
$wixOutDir = Join-Path $targetDir "wix"
$issPath = Join-Path $root "a2rs.iss"
$wxsPath = Join-Path $root "wix\main.wxs"

Ensure-Directory $distRoot
Ensure-Directory $innoOutDir
Ensure-Directory $wixOutDir

if (-not $SkipBuild) {
    Write-Section "Building release binary"

    $cargoArgs = @("build", "--$Profile")
    if ($FeatureSet) {
        $cargoArgs += @("--features", $FeatureSet)
    }

    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed."
    }
}

if (-not (Test-Path $exePath)) {
    throw "Executable not found: $exePath"
}

Write-Section "Creating distribution folder"
if (Test-Path $packageDir) {
    Remove-Item -Recurse -Force $packageDir
}
Ensure-Directory $packageDir

Copy-Item $exePath (Join-Path $packageDir $exeName) -Force

$optionalFiles = @(
    "README.md",
    "CHANGELOG.md",
    "LICENSE",
    "LICENSE.txt",
    "LICENSE.md"
)

foreach ($file in $optionalFiles) {
    $src = Join-Path $root $file
    if (Test-Path $src) {
        Copy-Item $src $packageDir -Force
    }
}

$runtimeDirs = @("roms", "disks", "docs")
foreach ($dir in $runtimeDirs) {
    $srcDir = Join-Path $root $dir
    if (Test-Path $srcDir) {
        Copy-Item $srcDir $packageDir -Recurse -Force
    }
}

if (-not $SkipZip) {
    Write-Section "Creating ZIP archive"
    if (Test-Path $zipPath) {
        Remove-Item $zipPath -Force
    }
    Compress-Archive -Path (Join-Path $packageDir "*") -DestinationPath $zipPath -CompressionLevel Optimal
    Write-Host "Portable ZIP created: $zipPath"
}

if (-not $SkipInno) {
    Write-Section "Building Inno Setup installer"

    $iscc = Find-Tool @("ISCC.exe", "ISCC")
    if (-not $iscc) {
        Write-Warning "Inno Setup compiler (ISCC) was not found in PATH. Skipping Inno Setup build."
    }
    elseif (-not (Test-Path $issPath)) {
        Write-Warning "a2rs.iss was not found: $issPath"
    }
    else {
        & $iscc "/DMyAppVersion=$version" "/DMyAppSourceDir=$packageDir" "/DMyOutputDir=$innoOutDir" $issPath
        if ($LASTEXITCODE -ne 0) {
            throw "Inno Setup build failed."
        }
        Write-Host "Inno Setup output directory: $innoOutDir"
    }
}

if (-not $SkipMsi) {
    Write-Section "Building MSI installer"

    $wix = Find-Tool @("wix.exe", "wix")
    if (-not $wix) {
        Write-Warning "WiX Toolset (wix) was not found in PATH. Skipping MSI build."
    }
    elseif (-not (Test-Path $wxsPath)) {
        Write-Warning "WiX source file was not found: $wxsPath"
    }
    elseif (-not (Test-WixUiExtensionInstalled -WixExe $wix)) {
        Write-Warning "WiX UI extension (WixToolset.UI.wixext) is not installed."
        Write-Warning "Run: wix extension add WixToolset.UI.wixext/6.0.2"
        Write-Warning "Skipping MSI build."
    }
    else {
        $msiPath = Join-Path $wixOutDir "a2rs-$version-windows-x64.msi"
        if (Test-Path $msiPath) {
            Remove-Item $msiPath -Force
        }

        $wixArgs = @(
            "build",
            "-arch", "x64",
            "-ext", "WixToolset.UI.wixext",
            "-d", "Version=$version",
            "-d", "SourceDir=$packageDir",
            "-d", "ProjectDir=$root",
            "-o", $msiPath,
            $wxsPath
        )

        & $wix @wixArgs
        if ($LASTEXITCODE -ne 0) {
            throw "MSI build failed."
        }
        Write-Host "MSI installer created: $msiPath"
    }
}

Write-Section "Build complete"
Write-Host "Output files:"
if (Test-Path $zipPath) { Write-Host "- $zipPath" }
Get-ChildItem $innoOutDir -Filter *.exe -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "- $($_.FullName)" }
Get-ChildItem $wixOutDir -Filter *.msi -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "- $($_.FullName)" }
