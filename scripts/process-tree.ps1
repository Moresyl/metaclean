function Get-VerifiedProcessTree {
  param(
    [Parameter(Mandatory = $true)][object[]]$Processes,
    [Parameter(Mandatory = $true)][ValidateRange(1, 2147483647)][int]$RootProcessId,
    [Parameter(Mandatory = $true)][datetime]$RootCreationDate
  )
  $byId = [Collections.Generic.Dictionary[int, object]]::new()
  foreach ($item in $Processes) {
    if ($item.CreationDate -isnot [datetime] -or $item.ProcessId -lt 1) { continue }
    $id = [int]$item.ProcessId
    if ($byId.ContainsKey($id)) { throw 'Duplicate process identifier in measurement snapshot' }
    $byId.Add($id, $item)
  }
  if (-not $byId.ContainsKey($RootProcessId)) { throw 'Application missing from measurement snapshot' }
  # CIM timestamps have microsecond precision; process handles expose 100 ns ticks.
  $expectedTicks = $RootCreationDate.ToUniversalTime().Ticks
  $expectedTicks -= $expectedTicks % 10
  $actualTicks = $byId[$RootProcessId].CreationDate.ToUniversalTime().Ticks
  $actualTicks -= $actualTicks % 10
  if ($actualTicks -ne $expectedTicks) { throw 'Application process identifier was reused' }

  $owned = [Collections.Generic.HashSet[int]]::new()
  [void]$owned.Add($RootProcessId)
  do {
    $added = $false
    foreach ($item in $byId.Values) {
      $parentId = [int]$item.ParentProcessId
      if (-not $owned.Contains($parentId) -or $owned.Contains([int]$item.ProcessId)) { continue }
      # A stale parent PID can point to an unrelated, newer process. A child
      # cannot predate its actual parent, even when the numeric PID matches.
      if ($item.CreationDate.ToUniversalTime() -lt $byId[$parentId].CreationDate.ToUniversalTime()) { continue }
      [void]$owned.Add([int]$item.ProcessId)
      $added = $true
    }
  } while ($added)

  foreach ($id in ($owned | Sort-Object)) {
    $item = $byId[$id]
    if ($null -eq $item.WorkingSetSize -or $null -eq $item.PrivatePageCount -or $item.WorkingSetSize -lt 0 -or $item.PrivatePageCount -lt 0) { throw 'Invalid process memory counters' }
    [pscustomobject]@{
      processId = $id
      parentProcessId = [int]$item.ParentProcessId
      name = [string]$item.Name
      creationDate = $item.CreationDate.ToUniversalTime().ToString('O')
      workingSetBytes = [long]$item.WorkingSetSize
      privateBytes = [long]$item.PrivatePageCount
    }
  }
}
