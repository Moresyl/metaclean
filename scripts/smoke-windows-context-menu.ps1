param(
  [Parameter(Mandatory = $true)]
  [string]$BinaryPath
)

$ErrorActionPreference = "Stop"
$binary = (Resolve-Path -LiteralPath $BinaryPath).Path
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
  throw "Context-menu smoke executable is missing: $binary"
}

$engineSource = Get-Content -LiteralPath (Join-Path $PSScriptRoot "..\src-tauri\src\engine.rs") -Raw
$allowlistMatch = [regex]::Match(
  $engineSource,
  'pub const SUPPORTED_EXTENSIONS:.*?= &\[(.*?)\];',
  [Text.RegularExpressions.RegexOptions]::Singleline
)
if (-not $allowlistMatch.Success) { throw "Could not read the native supported-extension allowlist" }
$extensions = @(
  [regex]::Matches($allowlistMatch.Groups[1].Value, '"([^"]+)"') |
    ForEach-Object { $_.Groups[1].Value }
)
if ($extensions.Count -eq 0 -or ($extensions | Sort-Object -Unique).Count -ne $extensions.Count) {
  throw "The native supported-extension allowlist is empty or contains duplicates"
}

function Get-VerbPath {
  param([Parameter(Mandatory = $true)][string]$Extension)
  return "HKCU:\Software\Classes\SystemFileAssociations\.$Extension\shell\MetaClean"
}

$preexisting = @($extensions | Where-Object { Test-Path -LiteralPath (Get-VerbPath $_) })
if ($preexisting.Count -gt 0) {
  throw "Refusing to replace $($preexisting.Count) existing MetaClean Explorer command key(s) during smoke testing"
}

$attemptedInstall = $false
try {
  $attemptedInstall = $true
  & $binary --install-context-menu
  if ($LASTEXITCODE -ne 0) { throw "Context-menu installation exited with code $LASTEXITCODE" }

  $expectedCommand = '"' + $binary + '" "%1"'
  foreach ($extension in $extensions) {
    $commandPath = Join-Path (Get-VerbPath $extension) "command"
    if (-not (Test-Path -LiteralPath $commandPath -PathType Container)) {
      throw "Explorer command is missing for .$extension"
    }
    $command = (Get-Item -LiteralPath $commandPath).GetValue("")
    if ($command -ne $expectedCommand) {
      throw "Explorer command for .$extension does not point to the candidate executable"
    }
  }
}
finally {
  if ($attemptedInstall) {
    & $binary --remove-context-menu
    if ($LASTEXITCODE -ne 0) { throw "Context-menu removal exited with code $LASTEXITCODE" }
  }
}

$remaining = @($extensions | Where-Object { Test-Path -LiteralPath (Get-VerbPath $_) })
if ($remaining.Count -gt 0) {
  throw "$($remaining.Count) MetaClean Explorer command key(s) remain after removal"
}
Write-Output "Explorer context-menu commands for $($extensions.Count) extensions installed, validated and removed cleanly."
