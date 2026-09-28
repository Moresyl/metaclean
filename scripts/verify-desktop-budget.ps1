param([Parameter(Mandatory = $true)][string]$Version, [ValidateSet(64, 256)][int]$SizeMiB = 64, [ValidateRange(1, 3)][int]$Iteration = 1)
$ErrorActionPreference = 'Stop'
if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or -not $env:RUNNER_TEMP) { throw 'Requires a disposable GitHub-hosted runner' }
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid stable version' }
if (Get-Process metaclean -ErrorAction SilentlyContinue) { throw 'Existing application process found' }
$root = Join-Path $env:RUNNER_TEMP "metaclean-memory-$PID"
$evidence = Join-Path $env:RUNNER_TEMP 'metaclean-memory-evidence'
if ((Test-Path -LiteralPath $root) -or (Test-Path -LiteralPath $evidence)) { throw 'Test directory already exists' }
New-Item -ItemType Directory -Path $root, $evidence | Out-Null
$release = gh release view "v$Version" --json isDraft,isPrerelease | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $release.isDraft -or $release.isPrerelease) { throw 'Expected a published stable release' }
$filename = "MetaClean_${Version}_x64_portable.zip"
gh release download "v$Version" --dir $root --pattern $filename --pattern SHASUMS256.txt
if ($LASTEXITCODE -ne 0) { throw 'Public package download failed' }
$archive = Join-Path $root $filename
$entry = @(Get-Content -LiteralPath (Join-Path $root 'SHASUMS256.txt') | Where-Object { $_ -match "^[a-f0-9]{64}  $([regex]::Escape($filename))$" })
if ($entry.Count -ne 1 -or (Get-FileHash -LiteralPath $archive).Hash -ne $entry[0].Substring(0, 64)) { throw 'Public package checksum mismatch' }
$install = Join-Path $root 'portable'
Expand-Archive -LiteralPath $archive -DestinationPath $install
$application = Join-Path $install 'MetaClean.exe'
if ((Get-Item -LiteralPath $application).VersionInfo.ProductVersion -ne $Version) { throw 'Executable version mismatch' }
$fixtures = Join-Path $root 'fixtures'
New-Item -ItemType Directory -Path $fixtures | Out-Null
$source = Join-Path $fixtures 'sample.txt'
$output = Join-Path $fixtures 'sample.cleaned.txt'
$bytes = [byte[]]::new($SizeMiB * 1024 * 1024)
[Array]::Fill[byte]($bytes, 97)
$cleanBytes = [byte[]]::new($bytes.Length - 3)
[Array]::Fill[byte]($cleanBytes, 97)
$cleanHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($cleanBytes))
$cleanBytes = $null
$bytes[0] = 0xE2; $bytes[1] = 0x80; $bytes[2] = 0x8B
$sourceHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
[IO.File]::WriteAllBytes($source, $bytes)
$bytes = $null
ConvertTo-Json -InputObject @($source) | Set-Content -LiteralPath (Join-Path $evidence 'sources.json') -Encoding utf8
$previousBrowserArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$applicationProcess = $null
$driver = $null
$driverStarted = $false
$policy = 'HKLM:\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
$policyCreated = $false
$processor = Get-CimInstance Win32_Processor | Select-Object -First 1
$operatingSystem = (Get-CimInstance Win32_OperatingSystem).Caption
try {
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222'
  if (Get-ItemProperty -LiteralPath $policy -Name 'MetaClean.exe' -ErrorAction SilentlyContinue) { throw 'Existing WebView debug policy found' }
  New-Item -Path $policy -Force | Out-Null
  New-ItemProperty -LiteralPath $policy -Name 'MetaClean.exe' -Value '--remote-debugging-port=9222' -PropertyType String | Out-Null
  $policyCreated = $true
  $applicationProcess = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
  $start = [Diagnostics.ProcessStartInfo]::new()
  $start.FileName = (Get-Command node).Source
  $start.UseShellExecute = $false
  $start.CreateNoWindow = $true
  $start.RedirectStandardOutput = $true
  $start.RedirectStandardError = $true
  foreach ($argument in @((Join-Path $PSScriptRoot 'verify-crash-recovery.mjs'), 'complete', $fixtures, $evidence)) { $start.ArgumentList.Add($argument) }
  $driver = [Diagnostics.Process]::new()
  $driver.StartInfo = $start
  if (-not $driver.Start()) { throw 'UI driver did not start' }
  $driverStarted = $true
  $stdout = $driver.StandardOutput.ReadToEndAsync()
  $stderr = $driver.StandardError.ReadToEndAsync()
  $timer = [Diagnostics.Stopwatch]::StartNew()
  $samples = 0
  $peakWorkingSet = 0L
  $peakPrivate = 0L
  $maxProcesses = 0
  while (-not $driver.HasExited) {
    if ($timer.Elapsed.TotalSeconds -gt 180) { throw 'Desktop measurement exceeded its time budget' }
    if ($applicationProcess.HasExited) { throw 'Application exited during measurement' }
    # Include only this application and its descendants, including WebView.
    # The fixture/controller and UI-driver processes are excluded.
    $inventory = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $owned = [Collections.Generic.HashSet[int]]::new()
    [void]$owned.Add($applicationProcess.Id)
    do {
      $added = $false
      foreach ($item in $inventory) {
        if ($owned.Contains([int]$item.ParentProcessId) -and $owned.Add([int]$item.ProcessId)) { $added = $true }
      }
    } while ($added)
    $workingSet = 0L
    $private = 0L
    $processCount = 0
    foreach ($item in @(Get-Process -Id ([int[]]@($owned)) -ErrorAction SilentlyContinue)) {
      try {
        $workingSet += $item.WorkingSet64
        $private += $item.PrivateMemorySize64
        $processCount++
      } catch [InvalidOperationException] { } # A child may exit between enumeration and sampling.
    }
    $peakWorkingSet = [Math]::Max($peakWorkingSet, $workingSet)
    $peakPrivate = [Math]::Max($peakPrivate, $private)
    $maxProcesses = [Math]::Max($maxProcesses, $processCount)
    $samples++
    [void]$driver.WaitForExit(200)
  }
  $timer.Stop()
  if ($driver.ExitCode -ne 0) { throw 'Released desktop workflow failed; inspect driver evidence' }
  if ($samples -eq 0 -or $maxProcesses -lt 2) { throw 'Application and WebView process tree was not observed' }
  if ((Get-FileHash -LiteralPath $source).Hash -ne $sourceHash) { throw 'Source content changed' }
  if ((Get-Item -LiteralPath $output).Length -ne $SizeMiB * 1024 * 1024 - 3 -or (Get-FileHash -LiteralPath $output).Hash -ne $cleanHash) { throw 'Cleaned output bytes did not match' }
  if (@(Get-ChildItem -LiteralPath $fixtures -File).Count -ne 2) { throw 'Unexpected extra fixture files remain' }
  $completed = Get-Content (Join-Path $evidence 'completed.json') -Raw | ConvertFrom-Json
  if ($completed.results[0].integrity.sourceSha256 -ne $sourceHash -or $completed.results[0].integrity.outputSha256 -ne $cleanHash) { throw 'Audit fingerprints did not match independent hashes' }
  [pscustomobject]@{ version = $Version; sizeMiB = $SizeMiB; iteration = $Iteration; architecture = 'x64'; operatingSystem = $operatingSystem; processor = $processor.Name; logicalProcessors = $processor.NumberOfLogicalProcessors; revision = $env:GITHUB_SHA; executableSha256 = (Get-FileHash -LiteralPath $application).Hash; sourceSha256 = $sourceHash.ToLowerInvariant(); outputSha256 = $cleanHash.ToLowerInvariant(); scanMs = $completed.scanMs; cleanMs = $completed.cleanMs; observedAggregateWorkingSetBytes = $peakWorkingSet; observedAggregatePrivateBytes = $peakPrivate; maxProcessCount = $maxProcesses; samples = $samples; elapsedMs = $timer.ElapsedMilliseconds; requestedWaitMs = 200; sourceAndOutputHashesVerified = $true; scope = 'Application plus descendants, including WebView; sampled working-set sums can double-count shared pages; not a guaranteed peak or upper bound' } | ConvertTo-Json | Set-Content (Join-Path $evidence 'results.json') -Encoding utf8
  Write-Output 'Published desktop memory measurement and large-file integrity checks passed'
} finally {
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousBrowserArguments
  if ($null -ne $driver) {
    if ($driverStarted) {
      if (-not $driver.HasExited) { $driver.Kill($true); $driver.WaitForExit() }
      $stdout.GetAwaiter().GetResult() | Set-Content (Join-Path $evidence 'driver-stdout.txt')
      $stderr.GetAwaiter().GetResult() | Set-Content (Join-Path $evidence 'driver-stderr.txt')
    }
    $driver.Dispose()
  }
  if ($null -ne $applicationProcess -and -not $applicationProcess.HasExited) { taskkill.exe /PID $applicationProcess.Id /T /F | Out-Null }
  if ($policyCreated) { Remove-ItemProperty -LiteralPath $policy -Name 'MetaClean.exe' }
}
