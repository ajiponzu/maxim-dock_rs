param(
    [string] $CompilerPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$dist = Join-Path $repoRoot 'dist'

if (-not $CompilerPath) {
    $candidates = @(
        (Join-Path $repoRoot 'target\InnoSetup\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
    )
    $CompilerPath = $candidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
}
if (-not $CompilerPath -or -not (Test-Path -LiteralPath $CompilerPath)) {
    throw 'Install Inno Setup 6.7.3 for non-commercial use, or pass -CompilerPath to ISCC.exe.'
}

cargo build --release --locked --manifest-path (Join-Path $repoRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw "Release build failed: $LASTEXITCODE" }

New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot 'target\release\maxim-dock_rs.exe') `
    -Destination (Join-Path $dist 'MaXImDock-v2-x86_64.exe') -Force

Push-Location $PSScriptRoot
try {
    & $CompilerPath 'MaXImDock.iss'
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup compilation failed: $LASTEXITCODE" }
}
finally {
    Pop-Location
}
