param(
  [Parameter(Mandatory = $true)][string]$PreviousVersion,
  [Parameter(Mandatory = $true)][string]$CurrentVersion
)

$ErrorActionPreference = 'Stop'
if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or -not $env:RUNNER_TEMP) {
  throw 'Requires a disposable GitHub-hosted runner'
}
foreach ($version in @($PreviousVersion, $CurrentVersion)) {
  if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid stable version' }
}
if ([version]$PreviousVersion -ge [version]$CurrentVersion) { throw 'Previous version must precede current version' }
$registryPaths = @(
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$existing = @(Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'MetaClean' })
if ($existing.Count -gt 0 -or (Get-Process metaclean -ErrorAction SilentlyContinue)) {
  throw 'Refusing to replace an existing installation or process'
}
$evidence = Join-Path $env:RUNNER_TEMP 'metaclean-msi-upgrade'
if (Test-Path -LiteralPath $evidence) { throw 'Evidence directory already exists' }
New-Item -ItemType Directory -Path $evidence | Out-Null
$assets = Join-Path $env:RUNNER_TEMP "metaclean-msi-assets-$PID"
$results = @()
$process = $null
$application = $null
$attempted = $false

foreach ($version in @($PreviousVersion, $CurrentVersion)) {
  $release = gh release view "v$version" --json isDraft,isPrerelease | ConvertFrom-Json
  if ($LASTEXITCODE -ne 0 -or $release.isDraft -or $release.isPrerelease) { throw 'Expected a published stable release' }
  $directory = Join-Path $assets $version
  gh release download "v$version" --dir $directory --pattern "MetaClean_${version}_x64_en-US.msi" --pattern SHASUMS256.txt
  if ($LASTEXITCODE -ne 0) { throw 'Public download failed' }
  $installer = Join-Path $directory "MetaClean_${version}_x64_en-US.msi"
  $manifest = Get-Content -LiteralPath (Join-Path $directory 'SHASUMS256.txt')
  $entry = @($manifest | Where-Object { $_ -match "^[a-f0-9]{64}  $([regex]::Escape([IO.Path]::GetFileName($installer)))$" })
  if ($entry.Count -ne 1 -or (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -ne $entry[0].Substring(0, 64)) {
    throw "Public MSI checksum mismatch for $version"
  }
}

try {
  $step = 0
  foreach ($version in @($PreviousVersion, $CurrentVersion, $PreviousVersion)) {
    $step++
    $installer = Join-Path $assets "$version/MetaClean_${version}_x64_en-US.msi"
    $log = Join-Path $evidence "step-$step-$version.log"
    $attempted = $true
    $install = Start-Process -FilePath 'msiexec.exe' -ArgumentList @('/i', "`"$installer`"", '/qn', '/norestart', '/L*v', "`"$log`"") -PassThru -Wait -WindowStyle Hidden
    if ($install.ExitCode -ne 0) { throw "MSI transition to $version failed: $($install.ExitCode)" }
    $entries = @(Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'MetaClean' })
    if ($entries.Count -ne 1 -or $entries[0].DisplayVersion -ne $version -or $entries[0].WindowsInstaller -ne 1) {
      throw "Expected one MSI registration at $version"
    }
    $application = Join-Path $entries[0].InstallLocation 'MetaClean.exe'
    $fileVersion = (Get-Item -LiteralPath $application).VersionInfo.ProductVersion
    if ($fileVersion -notmatch "^$([regex]::Escape($version))(?:\.0)?$") { throw 'Executable version mismatch' }
    $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 6
    $process.Refresh()
    if ($process.HasExited -or $process.MainWindowTitle -ne 'MetaClean') { throw "MSI application $version failed to launch" }
    Stop-Process -Id $process.Id -Force
    $process.WaitForExit()
    $process = $null
    $repairVerified = $false
    if ($version -eq $CurrentVersion) {
      $originalHash = (Get-FileHash -LiteralPath $application -Algorithm SHA256).Hash
      # Deliberately damage only this disposable runner's test installation.
      [IO.File]::WriteAllBytes($application, [byte[]]@(0, 1, 2, 3))
      if ((Get-FileHash -LiteralPath $application -Algorithm SHA256).Hash -eq $originalHash) { throw 'Damage fixture was not applied' }
      $repairLog = Join-Path $evidence 'repair.log'
      $repair = Start-Process -FilePath 'msiexec.exe' -ArgumentList @('/fa', "`"$installer`"", '/qn', '/norestart', '/L*v', "`"$repairLog`"") -PassThru -Wait -WindowStyle Hidden
      if ($repair.ExitCode -ne 0) { throw "MSI repair failed: $($repair.ExitCode)" }
      if ((Get-FileHash -LiteralPath $application -Algorithm SHA256).Hash -ne $originalHash) { throw 'MSI repair did not restore executable bytes' }
      $repaired = @(Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'MetaClean' })
      if ($repaired.Count -ne 1 -or $repaired[0].PSChildName -ne $entries[0].PSChildName -or $repaired[0].DisplayVersion -ne $version) { throw 'MSI repair changed package registration' }
      $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
      Start-Sleep -Seconds 6
      $process.Refresh()
      if ($process.HasExited -or $process.MainWindowTitle -ne 'MetaClean') { throw 'Repaired application failed to launch' }
      Stop-Process -Id $process.Id -Force
      $process.WaitForExit()
      $process = $null
      $repairVerified = $true
      Write-Output 'MSI repair restored the exact executable hash, registration and launch'
    }
    $results += [pscustomobject]@{ step = $step; version = $version; executableVersion = $fileVersion; productCode = $entries[0].PSChildName; launchPassed = $true; repairVerified = $repairVerified }
    $results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $evidence 'results.json') -Encoding utf8
    Write-Output "MSI installed and launched $version"
  }
} finally {
  if ($null -ne $process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force; $process.WaitForExit() }
  # The hosted-runner guard and empty initial registration make these test-owned.
  if ($attempted) {
    $entries = @(Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'MetaClean' -and $_.WindowsInstaller -eq 1 })
    foreach ($entry in $entries) {
      $log = Join-Path $evidence "uninstall-$($entry.PSChildName).log"
      $uninstall = Start-Process -FilePath 'msiexec.exe' -ArgumentList @('/x', $entry.PSChildName, '/qn', '/norestart', '/L*v', "`"$log`"") -PassThru -Wait -WindowStyle Hidden
      if ($uninstall.ExitCode -ne 0) { throw "MSI uninstall failed: $($uninstall.ExitCode)" }
    }
  }
}
$remaining = @(Get-ItemProperty $registryPaths -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'MetaClean' })
if ($remaining.Count -ne 0 -or ($application -and (Test-Path -LiteralPath $application))) { throw 'MSI removal left registration or executable' }
Write-Output 'MSI installation, upgrade, manual downgrade and removal passed'
