param(
  [switch]$SkipBuild,
  [ValidateSet("x64")]
  [string]$Architecture = "x64"
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$packageMetadata = Get-Content -LiteralPath (Join-Path $repositoryRoot "package.json") -Raw | ConvertFrom-Json
$configuration = "debug"
$bundleDirectory = Join-Path $repositoryRoot "src-tauri\target\$configuration\bundle"
$binaryPath = Join-Path $repositoryRoot "src-tauri\target\$configuration\metaclean.exe"

function Invoke-RepositoryCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,
    [Parameter(Mandatory = $true)]
    [string[]]$ArgumentList
  )

  Push-Location $repositoryRoot
  try {
    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) { throw "$FilePath exited with code $LASTEXITCODE" }
  }
  finally {
    Pop-Location
  }
}

if (-not $SkipBuild) {
  Write-Output "Building non-publishing Windows $configuration candidate for $Architecture..."
  Invoke-RepositoryCommand "pnpm.cmd" @("tauri", "build", "--debug", "--bundles", "nsis,msi")
}

if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
  throw "Candidate executable is missing: $binaryPath"
}
if (-not (Test-Path -LiteralPath $bundleDirectory -PathType Container)) {
  throw "Candidate bundle directory is missing: $bundleDirectory"
}

Write-Output "Packaging portable candidate..."
Invoke-RepositoryCommand "pwsh.exe" @(
  "-NoLogo", "-NoProfile", "-File", (Join-Path $repositoryRoot "scripts\package-windows-portable.ps1"),
  "-BinaryPath", $binaryPath,
  "-BundleDirectory", $bundleDirectory,
  "-Architecture", $Architecture
)

Write-Output "Running NSIS, MSI and portable smoke tests..."
Invoke-RepositoryCommand "pwsh.exe" @(
  "-NoLogo", "-NoProfile", "-File", (Join-Path $repositoryRoot "scripts\smoke-windows-installer.ps1"),
  "-BundleDirectory", $bundleDirectory
)
Invoke-RepositoryCommand "pwsh.exe" @(
  "-NoLogo", "-NoProfile", "-File", (Join-Path $repositoryRoot "scripts\smoke-windows-msi.ps1"),
  "-BundleDirectory", $bundleDirectory
)
Invoke-RepositoryCommand "pwsh.exe" @(
  "-NoLogo", "-NoProfile", "-File", (Join-Path $repositoryRoot "scripts\smoke-windows-portable.ps1"),
  "-BundleDirectory", $bundleDirectory,
  "-Architecture", $Architecture
)

Write-Output "Windows $($packageMetadata.version) $configuration preflight passed without publishing or pushing."
