# Regenerate src-tauri/icons/icon.ico from the transparent 512px master PNG.
#
# Why PNG-compressed frames: the previous BMP/DIB frames had an AND-mask /
# alpha-channel mismatch (alpha=0 but mask=opaque), so the Windows taskbar
# fell back to the mask and rendered an opaque black box. ICO with PNG
# payload frames (Vista+) keeps true 32-bit alpha end-to-end.
#
# Usage: powershell -File scripts/build-icon.ps1
# After replacing the .ico, rebuild the app binary (tauri-build embeds it via
# winres) and, if the taskbar still shows the old icon, clear the Windows icon
# cache or restart Explorer.

Add-Type -AssemblyName System.Drawing

$root   = Split-Path -Parent $PSScriptRoot
$master = [System.Drawing.Bitmap]::FromFile((Join-Path $root "src-tauri\icons\icon.png"))
$sizes  = @(16, 24, 32, 48, 64, 128, 256)

$pngPayloads = @()
foreach ($size in $sizes) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.InterpolationMode  = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode      = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode    = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($master, 0, 0, $size, $size)
    $g.Dispose()

    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    $pngPayloads += ,@{ Size = $size; Bytes = $ms.ToArray() }
    $ms.Dispose()
}
$master.Dispose()

$count = $pngPayloads.Count
$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)

# ICONDIR header
$bw.Write([uint16]0)       # reserved
$bw.Write([uint16]1)       # type: icon
$bw.Write([uint16]$count)  # image count

$offset = 6 + 16 * $count
foreach ($p in $pngPayloads) {
    $dim = if ($p.Size -ge 256) { 0 } else { $p.Size }
    $bw.Write([byte]$dim)                       # width (0 = 256)
    $bw.Write([byte]$dim)                       # height
    $bw.Write([byte]0)                          # palette colors
    $bw.Write([byte]0)                          # reserved
    $bw.Write([uint16]1)                        # color planes
    $bw.Write([uint16]32)                       # bits per pixel
    $bw.Write([uint32]$p.Bytes.Length)          # payload size
    $bw.Write([uint32]$offset)                  # payload offset
    $offset += $p.Bytes.Length
}
foreach ($p in $pngPayloads) {
    $bw.Write($p.Bytes)
}
$bw.Flush()

$out = Join-Path $root "src-tauri\icons\icon.ico"
[System.IO.File]::WriteAllBytes($out, $ms.ToArray())
$bw.Dispose()
$ms.Dispose()

"Wrote $out ($([IO.FileInfo]::new($out).Length) bytes, $count PNG frames: $($sizes -join ', '))"
