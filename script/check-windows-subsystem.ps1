[CmdletBinding()]
Param(
    [Parameter(Mandatory = $true)][string]$Path
)

$ErrorActionPreference = 'Stop'
$resolvedPath = (Resolve-Path -LiteralPath $Path).Path
$reader = [System.IO.BinaryReader]::new([System.IO.File]::OpenRead($resolvedPath))
try {
    $stream = $reader.BaseStream
    if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) {
        throw "Not a Windows PE executable: $resolvedPath"
    }
    $stream.Position = 0x3C
    [long]$peOffset = $reader.ReadUInt32()
    if ($peOffset -lt 64 -or $peOffset -gt ($stream.Length - 24)) {
        throw "Invalid PE header offset: $resolvedPath"
    }
    $stream.Position = $peOffset
    if ($reader.ReadUInt32() -ne 0x00004550) {
        throw "Missing PE signature: $resolvedPath"
    }
    $stream.Position = $peOffset + 20
    $optionalHeaderSize = $reader.ReadUInt16()
    $optionalHeaderOffset = $peOffset + 24
    if ($optionalHeaderSize -lt 70 -or ($optionalHeaderOffset + $optionalHeaderSize) -gt $stream.Length) {
        throw "Invalid PE optional header: $resolvedPath"
    }
    $stream.Position = $optionalHeaderOffset
    $magic = $reader.ReadUInt16()
    if ($magic -ne 0x10B -and $magic -ne 0x20B) {
        throw "Unsupported PE optional header: $resolvedPath"
    }
    # Subsystem is at byte 68 in both PE32 and PE32+ optional headers.
    $stream.Position = $optionalHeaderOffset + 68
    $subsystem = $reader.ReadUInt16()
    if ($subsystem -ne 2) {
        throw "Frame must use the Windows GUI subsystem (2), found $subsystem in $resolvedPath"
    }
    Write-Output "Verified Windows GUI subsystem (2): $resolvedPath"
}
finally {
    $reader.Dispose()
}
