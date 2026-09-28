param(
  [Parameter(Mandatory = $true)][string]$PreviousVersion,
  [Parameter(Mandatory = $true)][string]$CurrentVersion,
  [Parameter(Mandatory = $true)][string]$AssetDirectory,
  [Parameter(Mandatory = $true)][ValidateSet('x64', 'x86')][string]$Architecture
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
$originalBrowserArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222'
try {
  foreach ($version in @($PreviousVersion, $CurrentVersion, $PreviousVersion)) {
    $installer = Join-Path $AssetDirectory "$version/MetaClean_${version}_${Architecture}-setup.exe"
    $manifest = Get-Content -LiteralPath (Join-Path $AssetDirectory "$version/SHASUMS256.txt")
    $entry = @($manifest | Where-Object { $_ -match "^[a-f0-9]{64}  $([regex]::Escape([IO.Path]::GetFileName($installer)))$" })
    if ($entry.Count -ne 1 -or (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -ne $entry[0].Substring(0,64)) {
      throw "Public installer checksum mismatch for $version"
    }
    if ($version -eq $CurrentVersion) {
      $originalHash = (Get-FileHash -LiteralPath $application -Algorithm SHA256).Hash
      $damagedInstaller = Join-Path $env:RUNNER_TEMP "metaclean-truncated-$PID.exe"
      $bytes = [IO.File]::ReadAllBytes($installer)
      [IO.File]::WriteAllBytes($damagedInstaller, $bytes[0..([int]($bytes.Length / 2) - 1)])
      $damagedProcess = $null
      $rejected = $false
      try {
        try {
          $damagedProcess = Start-Process -FilePath $damagedInstaller -ArgumentList @('/S', "/D=$installRoot") -PassThru -WindowStyle Hidden
        } catch [System.ComponentModel.Win32Exception] {
          # Only an invalid executable is evidence of rejection; other launch errors fail the test.
          if ($_.Exception.NativeErrorCode -notin @(193, 216)) { throw }
          $rejected = $true
        }
        if ($null -ne $damagedProcess) {
          if (-not $damagedProcess.WaitForExit(30000)) { throw 'Damaged installer did not terminate within 30 seconds' }
          $rejected = $damagedProcess.ExitCode -ne 0
        }
      } finally {
        if ($null -ne $damagedProcess -and -not $damagedProcess.HasExited) { Stop-Process -Id $damagedProcess.Id -Force; $damagedProcess.WaitForExit() }
      }
      if (-not $rejected) { throw 'Damaged installer was not rejected' }
      if ((Get-FileHash -LiteralPath $application -Algorithm SHA256).Hash -ne $originalHash) { throw 'Failed installer changed the installed executable' }
      $registration = @(Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'MetaClean' })
      if ($registration.Count -ne 1 -or $registration[0].DisplayVersion -ne $PreviousVersion) { throw 'Failed installer changed registration' }
      $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
      Start-Sleep -Seconds 6
      $process.Refresh()
      if ($process.HasExited -or $process.MainWindowTitle -ne 'MetaClean') { throw 'Previous installation cannot launch after rejection' }
      node (Join-Path $PSScriptRoot 'verify-upgrade-storage.mjs') verify
      if ($LASTEXITCODE -ne 0) { throw 'User data changed after damaged installer rejection' }
      Stop-Process -Id $process.Id -Force
      $process.WaitForExit()
      $process = $null
      $results += [pscustomobject]@{ architecture = $Architecture; case = 'truncated-installer'; rejected = $true; previousHashUnchanged = $true; previousLaunchPassed = $true }
      Write-Output 'Truncated installer rejected; previous executable, registration and launch preserved'
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
    $storageMode = if ($results.Count -eq 0) { 'seed' } else { 'verify' }
    node (Join-Path $PSScriptRoot 'verify-upgrade-storage.mjs') $storageMode
    if ($LASTEXITCODE -ne 0) { throw "User data verification failed for $version" }
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit()
    $process = $null
    $results += [pscustomobject]@{ architecture = $Architecture; version = $version; executableVersion = $fileVersion; launchPassed = $true; storageVerified = $true }
    Write-Output "Installed and launched $version successfully"
  }
} finally {
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $originalBrowserArguments
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
