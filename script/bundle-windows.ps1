[CmdletBinding()]
Param(
    [Parameter()][Alias('i')][switch]$Install,
    [Parameter()][Alias('h')][switch]$Help,
    [Parameter()][switch]$PrepareOnly,
    [Parameter()][switch]$PackageOnly,
    [Parameter()][Alias('a')][string]$Architecture
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

function Help-Info {
    Write-Output "Usage: bundle-windows.ps1 [-Architecture x86_64] [-Install] [-PrepareOnly|-PackageOnly]"
    Write-Output "Build the Frame installer for Windows."
    Write-Output ""
    Write-Output "Options:"
    Write-Output "  -Architecture, -a Which architecture to build. Currently supported: x86_64"
    Write-Output "  -Install, -i      Run the installer after building."
    Write-Output "  -PrepareOnly      Build binaries and prepare installer inputs without signing."
    Write-Output "  -PackageOnly      Package prepared inputs without running Cargo."
    Write-Output "  -Help, -h         Show this help message."
}

if ($Help) {
    Help-Info
    exit 0
}

if ($PrepareOnly -and $PackageOnly) {
    throw '-PrepareOnly and -PackageOnly are mutually exclusive'
}

$RepoRoot = (Resolve-Path "$PSScriptRoot\..").Path

$OSArchitecture = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    'X64' { 'x86_64' }
    default { throw "Unsupported host architecture: $([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture)" }
}

$Architecture = if ($Architecture) { $Architecture } else { $OSArchitecture }
if ($Architecture -ne 'x86_64') {
    throw "Unsupported Windows architecture: $Architecture"
}

$Target = "$Architecture-pc-windows-msvc"
$TargetDir = if ($env:CARGO_TARGET_DIR) { (Resolve-Path $env:CARGO_TARGET_DIR).Path } else { "$RepoRoot\target" }
$ReleaseDir = "$TargetDir\$Target\release"
$InnoDir = "$RepoRoot\target\inno\$Architecture"
$InstallerPath = "$RepoRoot\target\Frame-$Architecture.exe"
$Version = (Select-String -Path "$RepoRoot\frame-app\Cargo.toml" -Pattern '^version = "(.+)"$').Matches[0].Groups[1].Value

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments
    )

    Write-Output "$Command $($Arguments -join ' ')"
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command exited with status $LASTEXITCODE"
    }
}

function Initialize-VsDevShell {
    $vsDevShell = 'C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1'
    if (Test-Path $vsDevShell) {
        Push-Location
        & $vsDevShell -Arch amd64 -HostArch amd64
        Pop-Location
    }
}

function Prepare-BundleDirectory {
    if (Test-Path $InnoDir) {
        Remove-Item -Path $InnoDir -Recurse -Force
    }
    New-Item -Path "$InnoDir\binaries" -ItemType Directory -Force | Out-Null

    Copy-Item -Path "$ReleaseDir\frame.exe" -Destination "$InnoDir\Frame.exe" -Force
    Copy-Item -Path "$ReleaseDir\frame-update-helper.exe" -Destination "$InnoDir\frame-update-helper.exe" -Force
    Copy-Item -Path "$RepoRoot\frame-app\resources\app-icons\icon.ico" -Destination "$InnoDir\app-icon.ico" -Force
    foreach ($binary in @("ffmpeg-$Target.exe", "ffprobe-$Target.exe")) {
        $source = "$RepoRoot\frame-app\resources\binaries\$binary"
        if (-not (Test-Path $source)) {
            throw "Missing runtime binary: $source"
        }
        Copy-Item -Path $source -Destination "$InnoDir\binaries\$binary" -Force
    }
}

function Build-Installer {
    $innoSetupPath = 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe'
    if (-not (Test-Path $innoSetupPath)) {
        throw "Inno Setup is required at $innoSetupPath"
    }

    if (Test-Path $InstallerPath) {
        Remove-Item $InstallerPath -Force
    }

    $definitions = @(
        "/dAppName=Frame",
        "/dAppSetupName=Frame-$Architecture",
        "/dAppVersion=$Version",
        "/dOutputDir=$RepoRoot\target",
        "/dResourcesDir=$InnoDir"
    )

    Invoke-Checked $innoSetupPath "$RepoRoot\inno\frame.iss" @definitions

    if (-not (Test-Path $InstallerPath)) {
        throw "Missing generated installer: $InstallerPath"
    }
}

if (-not $PackageOnly) {
    Initialize-VsDevShell
    Invoke-Checked rustup target add $Target
    Push-Location $RepoRoot
    try {
        Invoke-Checked cargo xtask setup-ffmpeg --platform win32 --arch $Architecture
    }
    finally {
        Pop-Location
    }
    Invoke-Checked cargo build --manifest-path "$RepoRoot\frame-app\Cargo.toml" --release --target $Target
    Invoke-Checked cargo build --manifest-path "$RepoRoot\frame-updater\Cargo.toml" --release --target $Target --bin frame-update-helper
    Prepare-BundleDirectory
}

if ($PrepareOnly) {
    Write-Output "Prepared unsigned installer inputs at $InnoDir"
    exit 0
}

Build-Installer

if ($Install) {
    Start-Process -FilePath $InstallerPath
}

Write-Output "Created $InstallerPath"
