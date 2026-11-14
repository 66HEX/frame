[CmdletBinding(PositionalBinding = $false)]
Param(
    [Parameter()][Alias('i')][switch]$Install,
    [Parameter()][Alias('h')][switch]$Help,
    [Parameter()][switch]$PrepareOnly,
    [Parameter()][switch]$PackageOnly,
    [Parameter(Position = 0)][Alias('a')][string]$Architecture,
    [Parameter()][string]$VsInstallPath,
    [Parameter()][string]$ISCCPath,
    [Parameter()][switch]$SkipDevShell
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

function Help-Info {
    Write-Output "Usage: bundle-windows.ps1 [-Architecture x86_64] [-Install] [-PrepareOnly|-PackageOnly] [-VsInstallPath <root>] [-ISCCPath <exe>] [-SkipDevShell]"
    Write-Output "Build the Frame installer for Windows."
    Write-Output ""
    Write-Output "Options:"
    Write-Output "  -Architecture, -a Which architecture to build. Currently supported: x86_64"
    Write-Output "  -Install, -i      Run the installer after building."
    Write-Output "  -PrepareOnly      Build binaries and prepare installer inputs without signing."
    Write-Output "  -PackageOnly      Package prepared inputs without running Cargo."
    Write-Output "  -VsInstallPath    Visual Studio installation root containing Common7\Tools\Launch-VsDevShell.ps1."
    Write-Output "  -ISCCPath         Path to Inno Setup's ISCC.exe. The ISCC_PATH environment variable is honoured too."
    Write-Output "  -SkipDevShell     Do not import the Visual Studio developer shell before building."
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

function Find-VsDevShell {
    param([string]$InstallPath)

    $relativePath = 'Common7\Tools\Launch-VsDevShell.ps1'

    # An explicit path is authoritative: do not silently fall back to another
    # Visual Studio installation when the requested one is unusable.
    if ($InstallPath) {
        $candidate = Join-Path $InstallPath $relativePath
        if (-not (Test-Path $candidate)) {
            throw "Launch-VsDevShell.ps1 not found at $candidate (from -VsInstallPath). Point -VsInstallPath at a Visual Studio installation root containing Common7\Tools."
        }
        return $candidate
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) {
        return $null
    }

    $arguments = @(
        '-latest',
        '-products', '*',
        '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
        '-property', 'installationPath'
    )

    # CONTRIBUTING documents the VS 2022 generation, but plain -latest can
    # return a newer Visual Studio. Prefer 2022 and only then accept any
    # installed version.
    try {
        $installations = @(& $vswhere @arguments -version '[17.0,18.0)')
        $installations += @(& $vswhere @arguments)
    }
    catch {
        return $null
    }
    $installations = $installations | Where-Object { $_ } | Select-Object -Unique

    foreach ($installation in $installations) {
        $candidate = Join-Path $installation.Trim() $relativePath
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    return $null
}

function Initialize-VsDevShell {
    $vsDevShell = Find-VsDevShell -InstallPath $VsInstallPath
    if (-not $vsDevShell) {
        Write-Warning 'Visual Studio C++ tools not found; continuing without a developer shell. Pass -VsInstallPath <root> if Visual Studio lives elsewhere, or -SkipDevShell to skip this step.'
        return
    }

    Write-Output "Using Visual Studio developer shell: $vsDevShell"
    Push-Location
    try {
        & $vsDevShell -Arch amd64 -HostArch amd64
    }
    finally {
        Pop-Location
    }
}

function Test-ISCCPath {
    param([string]$Path)

    if (-not $Path) {
        return $false
    }
    try {
        if ([IO.Path]::GetExtension($Path) -ne '.exe') {
            return $false
        }
        return Test-Path -LiteralPath $Path -PathType Leaf
    }
    catch {
        return $false
    }
}

function Find-ISCC {
    param([string]$ExplicitPath)

    if ($ExplicitPath) {
        if (-not (Test-ISCCPath $ExplicitPath)) {
            throw "ISCC.exe not found at $ExplicitPath (from -ISCCPath)"
        }
        return $ExplicitPath
    }

    if ($env:ISCC_PATH) {
        if (-not (Test-ISCCPath $env:ISCC_PATH)) {
            throw "ISCC.exe not found at $env:ISCC_PATH (from ISCC_PATH)"
        }
        return $env:ISCC_PATH
    }

    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command -and (Test-ISCCPath $command.Source)) {
        return $command.Source
    }

    $uninstallKeys = @(
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1',
        'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1'
    )
    foreach ($key in $uninstallKeys) {
        foreach ($valueName in @('InstallLocation', 'Inno Setup: App Path')) {
            $entry = Get-ItemProperty -Path $key -Name $valueName -ErrorAction SilentlyContinue
            if (-not $entry) {
                continue
            }
            $installLocation = $entry.$valueName
            if (-not $installLocation) {
                continue
            }
            $candidate = Join-Path $installLocation 'ISCC.exe'
            if (Test-ISCCPath $candidate) {
                return $candidate
            }
        }
    }

    $fallbacks = @(
        'C:\Program Files (x86)\Inno Setup 6\ISCC.exe',
        'C:\Program Files\Inno Setup 6\ISCC.exe',
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
    )
    foreach ($candidate in $fallbacks) {
        if (Test-ISCCPath $candidate) {
            return $candidate
        }
    }

    return $null
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
    if (-not $ISCC) {
        throw 'ISCC.exe has not been resolved; this is a bug in bundle-windows.ps1'
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

    Invoke-Checked $ISCC "$RepoRoot\inno\frame.iss" @definitions

    if (-not (Test-Path $InstallerPath)) {
        throw "Missing generated installer: $InstallerPath"
    }
}

# Resolve Inno Setup before any Cargo work so a missing compiler fails fast.
# -PrepareOnly stops at unsigned inputs and does not need Inno Setup.
$ISCC = $null
if (-not $PrepareOnly) {
    $ISCC = Find-ISCC -ExplicitPath $ISCCPath
    if (-not $ISCC) {
        $checked = @(
            'ISCC.exe on PATH',
            'HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1',
            'HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1',
            'C:\Program Files (x86)\Inno Setup 6\ISCC.exe',
            'C:\Program Files\Inno Setup 6\ISCC.exe',
            "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
        )
        throw ("Inno Setup 6 (ISCC.exe) not found. Checked:`n  " + ($checked -join "`n  ") + "`nPass -ISCCPath <ISCC.exe> or set ISCC_PATH if it lives elsewhere.`nInstall it with: choco install innosetup")
    }
    Write-Output "Using Inno Setup compiler: $ISCC"
}

if (-not $PackageOnly) {
    if (-not $SkipDevShell) {
        Initialize-VsDevShell
    }
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

# Check the application copied into the installer, including separately restored
# PackageOnly inputs. The installer and updater helper have different entrypoints.
& "$PSScriptRoot\check-windows-subsystem.ps1" -Path "$InnoDir\Frame.exe"

if ($PrepareOnly) {
    Write-Output "Prepared unsigned installer inputs at $InnoDir"
    exit 0
}

Build-Installer

if ($Install) {
    Start-Process -FilePath $InstallerPath
}

Write-Output "Created $InstallerPath"
