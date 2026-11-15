param(
    [Parameter(Mandatory=$true)][string]$Request,
    [Parameter(Mandatory=$true)][string]$Response
)
$ErrorActionPreference='Stop'
if([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'The QA process bridge requires native Windows'
}
Add-Type -Path (Join-Path $PSScriptRoot 'WindowsQaProcess.cs')
$requestData=[IO.File]::ReadAllText($Request) | ConvertFrom-Json
$timedOut=$false
$code=[WindowsQaProcess]::Run(
    [string]$requestData.executable,
    [string]$requestData.arguments,
    [string]$requestData.directory,
    [string]$requestData.stdout,
    [string]$requestData.stderr,
    [string]$requestData.ffmpeg,
    [string]$requestData.ffprobe,
    [int]$requestData.timeoutMs,
    [ref]$timedOut
)
$result=@{exitCode=$code; timedOut=$timedOut} | ConvertTo-Json
[IO.File]::WriteAllText($Response,$result,(New-Object System.Text.UTF8Encoding($false)))
