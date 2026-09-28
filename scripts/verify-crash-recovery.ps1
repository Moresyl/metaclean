param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or -not $env:RUNNER_TEMP) { throw 'Requires a disposable GitHub-hosted runner' }
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid stable version' }
if (Get-Process metaclean -ErrorAction SilentlyContinue) { throw 'Existing application process found' }
$root = Join-Path $env:RUNNER_TEMP "metaclean-crash-$PID"
$evidence = Join-Path $env:RUNNER_TEMP 'metaclean-crash-evidence'
if ((Test-Path -LiteralPath $root) -or (Test-Path -LiteralPath $evidence)) { throw 'Test directory already exists' }
New-Item -ItemType Directory -Path $root, $evidence | Out-Null
$release = gh release view "v$Version" --json isDraft,isPrerelease | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $release.isDraft -or $release.isPrerelease) { throw 'Expected a published stable release' }
$filename = "MetaClean_${Version}_x64_portable.zip"
gh release download "v$Version" --dir $root --pattern $filename --pattern SHASUMS256.txt
if ($LASTEXITCODE -ne 0) { throw 'Public package download failed' }
$archive = Join-Path $root $filename
$manifest = Get-Content -LiteralPath (Join-Path $root 'SHASUMS256.txt')
$entry = @($manifest | Where-Object { $_ -match "^[a-f0-9]{64}  $([regex]::Escape($filename))$" })
if ($entry.Count -ne 1 -or (Get-FileHash -LiteralPath $archive).Hash -ne $entry[0].Substring(0, 64)) { throw 'Public package checksum mismatch' }
$install = Join-Path $root 'portable'
Expand-Archive -LiteralPath $archive -DestinationPath $install
$application = Join-Path $install 'MetaClean.exe'
if ((Get-Item -LiteralPath $application).VersionInfo.ProductVersion -ne $Version) { throw 'Executable version mismatch' }
if (-not (Test-Path -LiteralPath (Join-Path $install 'metaclean-portable.marker'))) { throw 'Portable marker missing' }
$fixtures = Join-Path $root 'fixtures'
New-Item -ItemType Directory -Path $fixtures | Out-Null
$bytes = [byte[]]::new(8 * 1024 * 1024)
[Array]::Fill[byte]($bytes, 97)
$cleanBytes = [byte[]]::new($bytes.Length - 3)
[Array]::Fill[byte]($cleanBytes, 97)
$cleanHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($cleanBytes))
$cleanBytes = $null
$bytes[0] = 0xE2; $bytes[1] = 0x80; $bytes[2] = 0x8B
$sourceHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
$sources = @(0..63 | ForEach-Object {
  $source = Join-Path $fixtures "fixture-$_.txt"
  [IO.File]::WriteAllBytes($source, $bytes)
  $source
})
$sources | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'sources.json') -Encoding utf8
$policy = 'HKLM:\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
$policyCreated = $false
$process = $null
try {
  if (Get-ItemProperty -LiteralPath $policy -Name 'MetaClean.exe' -ErrorAction SilentlyContinue) { throw 'Existing WebView debug policy found' }
  New-Item -Path $policy -Force | Out-Null
  New-ItemProperty -LiteralPath $policy -Name 'MetaClean.exe' -Value '--remote-debugging-port=9222' -PropertyType String | Out-Null
  $policyCreated = $true
  $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
  node (Join-Path $PSScriptRoot 'verify-crash-recovery.mjs') start $fixtures $evidence
  if ($LASTEXITCODE -ne 0) { throw 'Could not reach partially completed cleanup' }
  # Only terminate the process tree started by this test on its disposable runner.
  taskkill.exe /PID $process.Id /T /F | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'Forced termination failed' }
  $process.WaitForExit()
  $process = $null
  foreach ($source in $sources) {
    if ((Get-FileHash -LiteralPath $source).Hash -ne $sourceHash) { throw 'Crash changed a source file' }
  }
  $outputs = @(Get-ChildItem -LiteralPath $fixtures -Filter '*.cleaned.txt' -File)
  if ($outputs.Count -eq 0 -or $outputs.Count -ge $sources.Count) { throw 'Did not interrupt a partially completed batch' }
  foreach ($output in $outputs) {
    if ($output.Length -ne $bytes.Length - 3 -or (Get-FileHash -LiteralPath $output.FullName).Hash -ne $cleanHash) { throw 'Committed output is incomplete or corrupt' }
  }
  $snapshot = @(Get-ChildItem -LiteralPath $fixtures -File | ForEach-Object { "$($_.Name):$((Get-FileHash -LiteralPath $_.FullName).Hash)" } | Sort-Object)
  [pscustomobject]@{ sourceFiles = $sources.Count; committedOutputs = $outputs.Count; otherFiles = $snapshot.Count - $sources.Count - $outputs.Count; sourceHashesPreserved = $true; committedOutputHashesVerified = $true } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'integrity-after-crash.json') -Encoding utf8
  foreach ($mode in @('recover', 'cleared')) {
    $process = Start-Process -FilePath $application -PassThru -WindowStyle Hidden
    node (Join-Path $PSScriptRoot 'verify-crash-recovery.mjs') $mode $fixtures $evidence
    if ($LASTEXITCODE -ne 0) { throw "Restart verification failed: $mode" }
    Start-Sleep -Seconds 3
    $afterRestart = @(Get-ChildItem -LiteralPath $fixtures -File | ForEach-Object { "$($_.Name):$((Get-FileHash -LiteralPath $_.FullName).Hash)" } | Sort-Object)
    if (Compare-Object $snapshot $afterRestart) { throw 'Restart changed files without user action' }
    taskkill.exe /PID $process.Id /T /F | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Test process cleanup failed' }
    $process.WaitForExit()
    $process = $null
  }
  [pscustomobject]@{ version = $Version; sourceFiles = $sources.Count; committedOutputs = $outputs.Count; otherFiles = $snapshot.Count - $sources.Count - $outputs.Count; sourceHashesPreserved = $true; committedOutputHashesVerified = $true; noAutomaticResume = $true; noticeClearedOnSecondRestart = $true } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'results.json') -Encoding utf8
  Write-Output 'Real cleanup interruption, source/output integrity and two restarts passed'
} finally {
  if ($null -ne $process -and -not $process.HasExited) { taskkill.exe /PID $process.Id /T /F | Out-Null }
  if ($policyCreated) { Remove-ItemProperty -LiteralPath $policy -Name 'MetaClean.exe' }
}
