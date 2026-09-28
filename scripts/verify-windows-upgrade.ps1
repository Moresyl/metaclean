param(
  [Parameter(Mandatory = $true)][string]$PreviousVersion,
  [Parameter(Mandatory = $true)][string]$CurrentVersion,
  [Parameter(Mandatory = $true)][string]$AssetDirectory
)

$ErrorActionPreference = "Stop"
foreach ($version in @($PreviousVersion, $CurrentVersion)) {
  if ($version -notmatch '^\d+\.\d+\.\d+$') { throw "Expected a stable numeric version" }
}
if ([version]$PreviousVersion -ge [version]$CurrentVersion) { throw "Previous version must precede current version" }
if ($env:RUNNER_ENVIRONMENT -ne "github-hosted" -or -not $env:RUNNER_TEMP) {
  throw "This installation test requires a disposable GitHub-hosted runner"
}
$registryPaths = @(
  "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
  "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
  "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*"
)
$existing = @(Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq "MetaClean" })
if ($existing.Count -gt 0 -or (Get-Process metaclean -ErrorAction SilentlyContinue)) {
  throw "Refusing to replace an existing MetaClean installation or process"
}
$installRoot = Join-Path $env:RUNNER_TEMP "metaclean-upgrade-$PID"
if (Test-Path -LiteralPath $installRoot) { throw "Test installation directory already exists" }
$application = Join-Path $installRoot "MetaClean.exe"
$process = $null
$results = @()
try {
  foreach ($version in @($PreviousVersion, $CurrentVersion, $PreviousVersion)) {
    $installer = Join-Path $AssetDirectory "$version/MetaClean_${version}_x64-setup.exe"
    $manifest = Get-Content -LiteralPath (Join-Path $AssetDirectory "$version/SHASUMS256.txt")
    $entry = @($manifest | Where-Object { $_ -match "^[a-f0-9]{64}  $([regex]::Escape([IO.Path]::GetFileName($installer)))$" })
    if ($entry.Count -ne 1 -or (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -ne $entry[0].Substring(0,64)) {
      throw "Public installer checksum mismatch for $version"
    }
    $install = Start-Process -FilePath $installer -ArgumentList @("/S", "/D=$installRoot") -PassThru -Wait -WindowStyle Hidden
    if ($install.ExitCode -ne 0) { throw "Installation of $version failed: $($install.ExitCode)" }
    $fileVersion = (Get-Item -LiteralPath $application).VersionInfo.ProductVersion
    if ($fileVersion -notmatch "^$([regex]::Escape($version))(?:\.0)?$") { throw "Installed version mismatch: $fileVersion" }
    $entry = @(Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq "MetaClean" })
    if ($entry.Count -ne 1 -or $entry[0].DisplayVersion -ne $version) { throw "Installation registration mismatch for $version" }
    $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 6
    $process.Refresh()
    if ($process.HasExited -or $process.MainWindowTitle -ne "MetaClean") { throw "Installed $version failed to launch" }
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit()
    $process = $null
    $results += [pscustomobject]@{ version = $version; executableVersion = $fileVersion; launchPassed = $true }
    Write-Output "Installed and launched $version successfully"
  }
} finally {
  if ($null -ne $process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force; $process.WaitForExit() }
  $uninstallers = @(Get-ChildItem -LiteralPath $installRoot -File -Filter '*uninstall*.exe' -ErrorAction SilentlyContinue)
  if ($uninstallers.Count -eq 1) {
    $uninstall = Start-Process -FilePath $uninstallers[0].FullName -ArgumentList '/S' -PassThru -Wait -WindowStyle Hidden
    if ($uninstall.ExitCode -ne 0) { throw "Uninstall failed: $($uninstall.ExitCode)" }
  }
}
if (Test-Path -LiteralPath $application) { throw "Application remains after uninstall" }
if (Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq "MetaClean" }) {
  throw "Registration remains after uninstall"
}
$results | ConvertTo-Json | Set-Content (Join-Path $env:RUNNER_TEMP 'metaclean-upgrade-results.json')
Write-Output "Public NSIS installation, upgrade, manual downgrade and uninstall passed"
