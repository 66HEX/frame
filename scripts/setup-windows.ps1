$ErrorActionPreference = "Stop"

$chocoPath = "C:\ProgramData\chocolatey\lib\ffmpeg-full\tools\ffmpeg\bin"
$destPath = Join-Path $PSScriptRoot "..\src-tauri"

# Define mappings (Source Filename -> Target Name with Triple)
$binaries = @{
    "ffmpeg.exe"  = "ffmpeg-x86_64-pc-windows-msvc.exe"
    "ffprobe.exe" = "ffprobe-x86_64-pc-windows-msvc.exe"
}

Write-Host "Setting up ffmpeg binaries for Windows..." -ForegroundColor Cyan

if (-not (Test-Path $chocoPath)) {
    Write-Warning "Chocolatey ffmpeg-full path not found at: $chocoPath"
    Write-Warning "Please install ffmpeg via chocolatey: choco install ffmpeg-full"
    Write-Warning "Or manually copy binaries to src-tauri named as: $($binaries.Values -join ', ')"
    exit 1
}

foreach ($bin in $binaries.Keys) {
    $source = Join-Path $chocoPath $bin
    $target = Join-Path $destPath $binaries[$bin]

    if (Test-Path $source) {
        Copy-Item -Path $source -Destination $target -Force
        Write-Host "Copied $bin to $target" -ForegroundColor Green
    } else {
        Write-Error "Could not find $bin in $chocoPath"
    }
}

Write-Host "Done!" -ForegroundColor Cyan
