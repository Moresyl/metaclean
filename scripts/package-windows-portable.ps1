param(
  [Parameter(Mandatory = $true)]
  [string]$BinaryPath,
  [Parameter(Mandatory = $true)]
  [string]$BundleDirectory,
  [Parameter(Mandatory = $true)]
  [ValidateSet("x64", "x86")]
  [string]$Architecture
)

$ErrorActionPreference = "Stop"
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Portable source executable is missing: $binary" }
New-Item -ItemType Directory -Path $BundleDirectory -Force | Out-Null
$bundle = (Resolve-Path -LiteralPath $BundleDirectory).Path
$packageMetadata = Get-Content -LiteralPath (Join-Path $PSScriptRoot "..\package.json") -Raw | ConvertFrom-Json
$archive = Join-Path $bundle "MetaClean_$($packageMetadata.version)_${Architecture}_portable.zip"
$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) "metaclean-portable-package-$PID-$Architecture"

try {
  if (Test-Path -LiteralPath $temporaryRoot) { Remove-Item -LiteralPath $temporaryRoot -Recurse -Force }
  New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
  Copy-Item -LiteralPath $binary -Destination (Join-Path $temporaryRoot "MetaClean.exe")
  Set-Content -LiteralPath (Join-Path $temporaryRoot "metaclean-portable.marker") -Value "portable" -Encoding ascii -NoNewline
  Copy-Item -LiteralPath (Join-Path $PSScriptRoot "..\README.md") -Destination $temporaryRoot
  Copy-Item -LiteralPath (Join-Path $PSScriptRoot "..\LICENSE") -Destination $temporaryRoot
  if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive -Force }
  Compress-Archive -LiteralPath (Get-ChildItem -LiteralPath $temporaryRoot).FullName -DestinationPath $archive -CompressionLevel Optimal
}
finally {
  if (Test-Path -LiteralPath $temporaryRoot) { Remove-Item -LiteralPath $temporaryRoot -Recurse -Force }
}

if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) { throw "Portable archive was not created: $archive" }
Write-Output "Created $archive"
