param(
  [Parameter(Mandatory = $true)]
  [string]$BundleDirectory,
  [Parameter(Mandatory = $true)]
  [ValidateSet("x64", "x86")]
  [string]$Architecture
)

$ErrorActionPreference = "Stop"
$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) "metaclean-portable-smoke-$PID-$Architecture"
$process = $null

try {
  $packageMetadata = Get-Content -LiteralPath (Join-Path $PSScriptRoot "..\package.json") -Raw | ConvertFrom-Json
  $archives = @(Get-ChildItem -LiteralPath $BundleDirectory -Recurse -File -Filter "MetaClean_$($packageMetadata.version)_${Architecture}_portable.zip")
  if ($archives.Count -ne 1) { throw "Expected one $Architecture portable archive, found $($archives.Count)" }
  if (Test-Path -LiteralPath $temporaryRoot) { Remove-Item -LiteralPath $temporaryRoot -Recurse -Force }
  Expand-Archive -LiteralPath $archives[0].FullName -DestinationPath $temporaryRoot
  $application = Join-Path $temporaryRoot "MetaClean.exe"
  if (-not (Test-Path -LiteralPath $application -PathType Leaf)) { throw "Portable executable is missing: $application" }
  $portableMarker = Join-Path $temporaryRoot "metaclean-portable.marker"
  if (-not (Test-Path -LiteralPath $portableMarker -PathType Leaf)) { throw "Portable marker is missing: $portableMarker" }
  $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
  Start-Sleep -Seconds 6
  $process.Refresh()
  if ($process.HasExited) { throw "Portable MetaClean exited during the smoke window" }
  if ($process.MainWindowTitle -ne "MetaClean") { throw "Unexpected portable window title: $($process.MainWindowTitle)" }
}
finally {
  if ($null -ne $process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force
    Wait-Process -Id $process.Id -Timeout 5 -ErrorAction SilentlyContinue
  }
  for ($attempt = 1; $attempt -le 10 -and (Test-Path -LiteralPath $temporaryRoot); $attempt++) {
    try { Remove-Item -LiteralPath $temporaryRoot -Recurse -Force -ErrorAction Stop }
    catch {
      if ($attempt -eq 10) { throw }
      Start-Sleep -Milliseconds 200
    }
  }
}

if (Test-Path -LiteralPath $temporaryRoot) { throw "Portable smoke directory remains after cleanup" }
Write-Output "Portable $Architecture package launched for six seconds and cleaned up."
