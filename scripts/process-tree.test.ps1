$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'process-tree.ps1')
$created = [datetime]::new(2026, 9, 29, 0, 0, 0, [DateTimeKind]::Utc)
function New-TestProcess([int]$Id, [int]$Parent, [int]$Offset) {
  [pscustomobject]@{ ProcessId = $Id; ParentProcessId = $Parent; CreationDate = $created.AddSeconds($Offset); Name = "process-$Id"; WorkingSetSize = 100L; PrivatePageCount = 50L }
}
function Assert-Rejected([scriptblock]$Action, [string]$Message) {
  $failed = $false
  try { & $Action | Out-Null } catch { $failed = $true }
  if (-not $failed) { throw $Message }
}
$root = New-TestProcess 10 1 0
$child = New-TestProcess 20 10 1
$grandchild = New-TestProcess 30 20 2
$staleChild = New-TestProcess 40 10 -10
$unrelated = New-TestProcess 50 40 -5
$reusedParent = New-TestProcess 60 20 0
$cycleA = New-TestProcess 70 80 3
$cycleB = New-TestProcess 80 70 3
$unknownCreated = New-TestProcess 90 10 1
$unknownCreated.CreationDate = $null
$rows = @($grandchild, $staleChild, $unrelated, $child, $reusedParent, $root, $cycleA, $cycleB, $unknownCreated)
$tree = @(Get-VerifiedProcessTree $rows 10 $created)
if (($tree.processId -join ',') -ne '10,20,30') { throw 'Incorrect descendant membership or ordering' }
if (($tree.workingSetBytes | Measure-Object -Sum).Sum -ne 300 -or ($tree.privateBytes | Measure-Object -Sum).Sum -ne 150) { throw 'Snapshot counters were not preserved' }
if ($tree[1].creationDate -ne $child.CreationDate.ToString('O') -or $tree[1].name -ne 'process-20') { throw 'Process identity evidence missing' }
$preciseHandleDate = $created.AddTicks(9)
if (@(Get-VerifiedProcessTree @($root) 10 $preciseHandleDate).Count -ne 1) { throw 'CIM timestamp precision was not handled' }
Assert-Rejected { Get-VerifiedProcessTree @($root) 10 $created.AddSeconds(1) } 'Reused root PID accepted'
Assert-Rejected { Get-VerifiedProcessTree @($child) 10 $created } 'Missing root accepted'
Assert-Rejected { Get-VerifiedProcessTree @($root, $root) 10 $created } 'Duplicate PID accepted'
$invalid = New-TestProcess 10 1 0
$invalid.WorkingSetSize = -1
Assert-Rejected { Get-VerifiedProcessTree @($invalid) 10 $created } 'Negative working set accepted'
$invalid.WorkingSetSize = 0
$invalid.PrivatePageCount = -1
Assert-Rejected { Get-VerifiedProcessTree @($invalid) 10 $created } 'Negative private bytes accepted'
$invalid.PrivatePageCount = $null
Assert-Rejected { Get-VerifiedProcessTree @($invalid) 10 $created } 'Missing memory counter accepted'
$current = Get-CimInstance Win32_Process -Filter "ProcessId = $PID"
$actual = @(Get-VerifiedProcessTree @($current) $PID (Get-Process -Id $PID).StartTime)
if ($actual.Count -ne 1 -or $actual[0].workingSetBytes -le 0 -or $actual[0].privateBytes -le 0) { throw 'Live CIM identity or counters failed validation' }
Write-Output 'Verified process ancestry, PID reuse, timestamp precision, evidence and memory counters'
