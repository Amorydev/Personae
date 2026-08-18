<#
  tint-ico.ps1 — best-effort per-profile tinted .ico (Windows).

  The macOS tool tints via a compiled Swift/CoreImage helper (icontint); that is
  macOS-only. Here we tint with System.Drawing (GDI+), which ships with Windows
  PowerShell 5.1 and .NET. The base icon is pulled from Claude.exe itself via
  ExtractAssociatedIcon (typically 32x32 — low-res; higher-res needs shell APIs,
  tracked as a verify item). We multiply-blend the accent colour over the glyph,
  preserve alpha, then wrap the PNG in a minimal ICO container (PNG-in-ICO is
  valid on Vista+).

  On ANY failure this exits non-zero and the Rust caller falls back to Claude's
  own (un-tinted) icon — a profile is never blocked on icon tinting.

  Usage: powershell -NoProfile -ExecutionPolicy Bypass -File tint-ico.ps1 -Exe <claude.exe> -Out <out.ico> -Hex RRGGBB
#>
param(
  [Parameter(Mandatory = $true)][string]$Exe,
  [Parameter(Mandatory = $true)][string]$Out,
  [Parameter(Mandatory = $true)][string]$Hex
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$ico = [System.Drawing.Icon]::ExtractAssociatedIcon($Exe)
$bmp = $ico.ToBitmap()

$r = [Convert]::ToInt32($Hex.Substring(0, 2), 16)
$g = [Convert]::ToInt32($Hex.Substring(2, 2), 16)
$b = [Convert]::ToInt32($Hex.Substring(4, 2), 16)

for ($y = 0; $y -lt $bmp.Height; $y++) {
  for ($x = 0; $x -lt $bmp.Width; $x++) {
    $px = $bmp.GetPixel($x, $y)
    if ($px.A -eq 0) { continue }
    $nr = [int]($px.R * $r / 255)
    $ng = [int]($px.G * $g / 255)
    $nb = [int]($px.B * $b / 255)
    $bmp.SetPixel($x, $y, [System.Drawing.Color]::FromArgb($px.A, $nr, $ng, $nb))
  }
}

$ms = New-Object System.IO.MemoryStream
$bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
$png = $ms.ToArray()

# 0 in the width/height byte means "256"; otherwise the actual size.
$w = if ($bmp.Width  -ge 256) { 0 } else { $bmp.Width }
$h = if ($bmp.Height -ge 256) { 0 } else { $bmp.Height }

$fs = [System.IO.File]::Open($Out, [System.IO.FileMode]::Create)
$bw = New-Object System.IO.BinaryWriter($fs)
try {
  # ICONDIR: reserved(0), type(1=icon), count(1)
  $bw.Write([UInt16]0); $bw.Write([UInt16]1); $bw.Write([UInt16]1)
  # ICONDIRENTRY: w, h, colorCount(0), reserved(0), planes(1), bpp(32),
  #               bytesInRes, imageOffset(22)
  $bw.Write([byte]$w); $bw.Write([byte]$h); $bw.Write([byte]0); $bw.Write([byte]0)
  $bw.Write([UInt16]1); $bw.Write([UInt16]32)
  $bw.Write([UInt32]$png.Length); $bw.Write([UInt32]22)
  $bw.Write($png)
  $bw.Flush()
} finally {
  $bw.Dispose(); $fs.Dispose()
}
Write-Output "Tinted ico written: $Out (#$Hex)"
