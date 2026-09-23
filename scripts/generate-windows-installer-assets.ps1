param()

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$outputDirectory = Join-Path $repositoryRoot "src-tauri\windows\installer"

Add-Type -AssemblyName System.Drawing

function New-InstallerBitmap {
  param(
    [Parameter(Mandatory = $true)][int]$Width,
    [Parameter(Mandatory = $true)][int]$Height,
    [Parameter(Mandatory = $true)][scriptblock]$Draw,
    [Parameter(Mandatory = $true)][string]$OutputPath
  )

  $bitmap = [Drawing.Bitmap]::new($Width, $Height, [Drawing.Imaging.PixelFormat]::Format24bppRgb)
  $graphics = [Drawing.Graphics]::FromImage($bitmap)
  try {
    $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.TextRenderingHint = [Drawing.Text.TextRenderingHint]::ClearTypeGridFit
    & $Draw $graphics
    $bitmap.Save($OutputPath, [Drawing.Imaging.ImageFormat]::Bmp)
  }
  finally {
    $graphics.Dispose()
    $bitmap.Dispose()
  }
}

function New-Brush([string]$Hex) {
  return [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml($Hex))
}

function New-Pen([string]$Hex, [float]$Width = 1) {
  return [Drawing.Pen]::new([Drawing.ColorTranslator]::FromHtml($Hex), $Width)
}

function New-Font([float]$Size, [Drawing.FontStyle]$Style = [Drawing.FontStyle]::Regular) {
  return [Drawing.Font]::new("Segoe UI", $Size, $Style, [Drawing.GraphicsUnit]::Pixel)
}

function Draw-Mark {
  param(
    [Parameter(Mandatory = $true)][Drawing.Graphics]$Graphics,
    [Parameter(Mandatory = $true)][float]$X,
    [Parameter(Mandatory = $true)][float]$Y,
    [Parameter(Mandatory = $true)][float]$Size,
    [Parameter(Mandatory = $true)][string]$Color
  )

  $pen = New-Pen $Color ([Math]::Max(2, $Size * 0.09))
  try {
    $pen.StartCap = [Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [Drawing.Drawing2D.LineCap]::Round
    $Graphics.DrawLine($pen, $X, $Y + $Size, $X, $Y)
    $Graphics.DrawLine($pen, $X, $Y, $X + ($Size * 0.5), $Y + ($Size * 0.55))
    $Graphics.DrawLine($pen, $X + ($Size * 0.5), $Y + ($Size * 0.55), $X + $Size, $Y)
    $Graphics.DrawLine($pen, $X + $Size, $Y, $X + $Size, $Y + $Size)
  }
  finally {
    $pen.Dispose()
  }
}

New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

$dark = "#161616"
$canvas = "#202020"
$surface = "#2B2B2B"
$light = "#F0F0F0"
$line = "#D6D6D6"
$text = "#E8E8E8"
$muted = "#A3A3A3"
$brand = "#F4F4F4"

New-InstallerBitmap 164 314 {
  param($g)
  $surfaceBrush = New-Brush $surface
  $textBrush = New-Brush $text
  $mutedBrush = New-Brush $muted
  $brandBrush = New-Brush $brand
  $titleFont = New-Font 17 ([Drawing.FontStyle]::Bold)
  $labelFont = New-Font 8 ([Drawing.FontStyle]::Bold)
  $copyFont = New-Font 9
  $linePen = New-Pen "#3B3B3B"
  try {
    $g.Clear([Drawing.ColorTranslator]::FromHtml($dark))
    $g.DrawLine($linePen, 163, 0, 163, 314)
    Draw-Mark $g 20 22 24 $brand
    $g.DrawString("MetaClean", $titleFont, $textBrush, 20, 58)
    $g.DrawString("LOCAL FILE PRIVACY", $labelFont, $mutedBrush, 20, 84)
    $g.FillRectangle($surfaceBrush, 20, 112, 124, 1)
    $g.FillRectangle($brandBrush, 20, 130, 4, 4)
    $g.DrawString("Scan locally", $copyFont, $textBrush, 32, 124)
    $g.FillRectangle($brandBrush, 20, 157, 4, 4)
    $g.DrawString("Verify before write", $copyFont, $textBrush, 32, 151)
    $g.FillRectangle($brandBrush, 20, 184, 4, 4)
    $g.DrawString("Keep recovery paths", $copyFont, $textBrush, 32, 178)
    $g.DrawString("FILES STAY ON DEVICE", $labelFont, $mutedBrush, 20, 279)
  }
  finally {
    @($surfaceBrush, $textBrush, $mutedBrush, $brandBrush, $titleFont, $labelFont, $copyFont, $linePen) |
      ForEach-Object { $_.Dispose() }
  }
} (Join-Path $outputDirectory "nsis-sidebar.bmp")

New-InstallerBitmap 150 57 {
  param($g)
  $textBrush = New-Brush $text
  $mutedBrush = New-Brush $muted
  $titleFont = New-Font 12 ([Drawing.FontStyle]::Bold)
  $labelFont = New-Font 7 ([Drawing.FontStyle]::Bold)
  try {
    $g.Clear([Drawing.ColorTranslator]::FromHtml($dark))
    Draw-Mark $g 15 13 22 $brand
    $g.DrawString("MetaClean", $titleFont, $textBrush, 49, 12)
    $g.DrawString("LOCAL-FIRST", $labelFont, $mutedBrush, 50, 31)
  }
  finally {
    @($textBrush, $mutedBrush, $titleFont, $labelFont) | ForEach-Object { $_.Dispose() }
  }
} (Join-Path $outputDirectory "nsis-header.bmp")

New-InstallerBitmap 493 58 {
  param($g)
  $rail = New-Brush $dark
  $textBrush = New-Brush $text
  $linePen = New-Pen $line
  $titleFont = New-Font 12 ([Drawing.FontStyle]::Bold)
  try {
    $g.Clear([Drawing.ColorTranslator]::FromHtml($light))
    $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::None
    $g.FillRectangle($rail, 343, 0, 150, 57)
    $g.DrawLine($linePen, 0, 57, 493, 57)
    $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    Draw-Mark $g 362 16 20 $brand
    $g.DrawString("MetaClean", $titleFont, $textBrush, 397, 16)
  }
  finally {
    @($rail, $textBrush, $linePen, $titleFont) | ForEach-Object { $_.Dispose() }
  }
} (Join-Path $outputDirectory "wix-banner.bmp")

New-InstallerBitmap 493 312 {
  param($g)
  $rail = New-Brush $dark
  $surfaceBrush = New-Brush $canvas
  $textBrush = New-Brush $text
  $mutedBrush = New-Brush $muted
  $brandBrush = New-Brush $brand
  $titleFont = New-Font 17 ([Drawing.FontStyle]::Bold)
  $labelFont = New-Font 8 ([Drawing.FontStyle]::Bold)
  $copyFont = New-Font 9
  $linePen = New-Pen $line
  try {
    $g.Clear([Drawing.ColorTranslator]::FromHtml($light))
    $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::None
    $g.FillRectangle($rail, 0, 0, 164, 312)
    $g.FillRectangle($surfaceBrush, 20, 111, 124, 1)
    $g.DrawLine($linePen, 164, 0, 164, 312)
    $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    Draw-Mark $g 20 22 24 $brand
    $g.DrawString("MetaClean", $titleFont, $textBrush, 20, 58)
    $g.DrawString("LOCAL FILE PRIVACY", $labelFont, $mutedBrush, 20, 84)
    $g.FillRectangle($brandBrush, 20, 130, 4, 4)
    $g.DrawString("Scan locally", $copyFont, $textBrush, 32, 124)
    $g.FillRectangle($brandBrush, 20, 157, 4, 4)
    $g.DrawString("Verify before write", $copyFont, $textBrush, 32, 151)
    $g.FillRectangle($brandBrush, 20, 184, 4, 4)
    $g.DrawString("Keep recovery paths", $copyFont, $textBrush, 32, 178)
    $g.DrawString("FILES STAY ON DEVICE", $labelFont, $mutedBrush, 20, 277)
  }
  finally {
    @($rail, $surfaceBrush, $textBrush, $mutedBrush, $brandBrush, $titleFont, $labelFont, $copyFont, $linePen) |
      ForEach-Object { $_.Dispose() }
  }
} (Join-Path $outputDirectory "wix-dialog.bmp")

Write-Output "Generated branded NSIS and WiX bitmaps in $outputDirectory"
