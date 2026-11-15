param([string]$Runs=(Join-Path (Split-Path -Parent $PSScriptRoot) 'runs'))
$ErrorActionPreference='Stop'
if([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    'NOT_RUN: native Windows Job Object test'
    exit 0
}

[void](New-Item -ItemType Directory -Path $Runs -Force)
$Runs=(Resolve-Path -LiteralPath $Runs).Path
$root=Join-Path $Runs ("frame job self-test Zażółć's space "+[guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $root)
$summary=Join-Path $Runs 'windows-job-self-test.json'
$utf8Bom=New-Object System.Text.UTF8Encoding($true)
$utf8=New-Object System.Text.UTF8Encoding($false)
$cases=New-Object 'Collections.Generic.List[string]'

function Write-Report($Report) {
    $json=$Report | ConvertTo-Json -Depth 5
    [IO.File]::WriteAllText((Join-Path $root 'result.json'),$json,$utf8)
    [IO.File]::WriteAllText($summary,$json,$utf8)
    $json
}

try {
    $helper=Join-Path (Split-Path -Parent $PSScriptRoot) 'WindowsQaProcess.cs'
    Add-Type -Path $helper
    $shell=(Get-Process -Id $PID).Path
    $childScript=Join-Path $root 'child.ps1'
    $readyFile=Join-Path $root 'child.ready'
    $child=@'
$ErrorActionPreference='Stop'
[Console]::Out.WriteLine('child holds stdout')
[Console]::Out.Flush()
[IO.File]::WriteAllText('READYFILE','ready')
Start-Sleep -Seconds 60
'@
    $child=$child.Replace('READYFILE',$readyFile.Replace("'","''"))
    [IO.File]::WriteAllText($childScript,$child,$utf8Bom)
    $parentScript=Join-Path $root 'parent.ps1'
    $parent=@'
$ErrorActionPreference='Stop'
$child = Start-Process -FilePath (Get-Process -Id $PID).Path -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File "CHILD"' -NoNewWindow -PassThru
$wait=[Diagnostics.Stopwatch]::StartNew()
while(-not (Test-Path -LiteralPath 'READYFILE')) {
    if($child.HasExited -or $wait.ElapsedMilliseconds -gt 5000){throw 'Child did not reach its inherited-output handshake'}
    Start-Sleep -Milliseconds 20
}
if($child.HasExited){throw 'Child exited before parent exit could exercise job cleanup'}
[IO.File]::WriteAllText('PIDFILE',[string]$child.Id)
'@
    $parent=$parent.Replace('CHILD',$childScript.Replace("'","''")).Replace('READYFILE',$readyFile.Replace("'","''")).Replace('PIDFILE',(Join-Path $root 'child.pid').Replace("'","''"))
    # Windows PowerShell 5.1 requires a BOM to read the embedded Unicode paths.
    [IO.File]::WriteAllText($parentScript,$parent,$utf8Bom)
    $out=Join-Path $root 'stdout'; $err=Join-Path $root 'stderr'
    $clock=[Diagnostics.Stopwatch]::StartNew()
    $code=[WindowsQaProcess]::Run($shell,('-NoProfile -ExecutionPolicy Bypass -File "'+$parentScript+'"'),$root,$out,$err,'unused','unused',10000)
    if($code -ne 0 -or $clock.Elapsed.TotalSeconds -gt 16){throw 'Exited parent with inherited stdout was not bounded'}
    if(-not (Test-Path -LiteralPath $readyFile) -or -not ([IO.File]::ReadAllText($out).Contains('child holds stdout'))){throw 'Child did not execute and write through its inherited output handle'}
    [void]$cases.Add('exited-parent-inherited-output')
    $childId=[int]([IO.File]::ReadAllText((Join-Path $root 'child.pid')))
    if(Get-Process -Id $childId -ErrorAction SilentlyContinue){throw 'Owned child survived parent exit'}
    [void]$cases.Add('owned-child-cleanup')
    [void]$cases.Add('unicode-apostrophe-space-path')
    $out=Join-Path $root 'timeout.stdout'; $err=Join-Path $root 'timeout.stderr'
    $clock.Restart()
    $code=[WindowsQaProcess]::Run($shell,'-NoProfile -Command "Start-Sleep -Seconds 60"',$root,$out,$err,'unused','unused',500)
    if($code -ne 124 -or $clock.Elapsed.TotalSeconds -gt 7){throw 'Timeout did not terminate owned job within deadline'}
    [void]$cases.Add('timeout')
    Write-Report @{status='PASS'; cases=$cases.ToArray(); shell=$shell; version=$PSVersionTable.PSVersion.ToString(); helperSha256=(Get-FileHash -LiteralPath $helper -Algorithm SHA256).Hash.ToLowerInvariant(); reportDirectory=$root}
} catch {
    Write-Report @{status='FAIL'; cases=$cases.ToArray(); error=$_.Exception.Message; version=$PSVersionTable.PSVersion.ToString(); reportDirectory=$root}
    throw
}
