# Controlled release check. Never stop an existing user instance by name.
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$exePath = Join-Path $repoRoot 'dist\MaXIMDock.exe'
if (Get-Process -Name MaXIMDock -ErrorAction SilentlyContinue) {
    throw 'Close existing MaXIMDock instances before this controlled test.'
}
$bytes = [IO.File]::ReadAllBytes($exePath)
$peOffset = [BitConverter]::ToInt32($bytes, 0x3c)
$subsystem = [BitConverter]::ToUInt16($bytes, $peOffset + 24 + 68)
if ($subsystem -ne 2) { throw "Expected Windows GUI subsystem, got $subsystem" }
$testDir = Join-Path $repoRoot ('target\parent-control-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testDir | Out-Null
$ownedProcess = $null
try {
    $configPath = Join-Path $testDir 'pid.toml'
    $ownedProcess = Start-Process -FilePath $exePath -ArgumentList @('--config', ('"' + $configPath + '"')) -PassThru
    Start-Sleep -Milliseconds 1000
    $ownedProcess.Refresh()
    if ($ownedProcess.HasExited -or $ownedProcess.ProcessName -ne 'MaXIMDock') { throw 'PassThru did not return the running Dock process.' }
    Stop-Process -Id $ownedProcess.Id -Force
    if (-not $ownedProcess.WaitForExit(5000)) { throw 'PID termination timed out.' }
    $ownedProcess = $null
    $configPath = Join-Path $testDir 'name.toml'
    Start-Process -FilePath $exePath -ArgumentList @('--config', ('"' + $configPath + '"'))
    Start-Sleep -Milliseconds 1000
    $instances = @(Get-Process -Name MaXIMDock -ErrorAction SilentlyContinue)
    if ($instances.Count -ne 1 -or $instances[0].Path -ne $exePath) { throw 'Unexpected process set; refusing name-based termination.' }
    $ownedProcess = $instances[0]
    Stop-Process -Name MaXIMDock -Force
    if (-not $ownedProcess.WaitForExit(5000)) { throw 'Name termination timed out.' }
    $ownedProcess = $null
    Write-Output 'PARENT_CONTROL_PASS: GUI subsystem=2; Start-Process PID stop and no-PassThru name stop succeeded.'
}
finally {
    if ($ownedProcess) {
        $ownedProcess.Refresh()
        if (-not $ownedProcess.HasExited) { Stop-Process -Id $ownedProcess.Id -Force }
        $ownedProcess.Dispose()
    }
}
