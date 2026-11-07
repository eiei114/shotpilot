# Build the desktop app (and the CLI next to it) and open its window.
#
#   run.ps1                      release build, opens the home
#   run.ps1 path\to\shots.yaml   opens that project (shots.yaml or its folder)
#   run.ps1 -Dev                 debug build (a console window shows the logs)
#   run.ps1 -NoBuild             launch the last build as it is
param(
    [string]$Project = "",
    [switch]$Dev,
    [switch]$NoBuild
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$profileDir = if ($Dev) { 'debug' } else { 'release' }
$exe = Join-Path $root "target\$profileDir\shotpilot-app.exe"

if ($Project) {
    if (-not (Test-Path $Project)) { Write-Error "project not found: $Project" }
    $Project = (Resolve-Path $Project).Path
}

# Windows locks a running exe, so the build cannot replace it: close the open window first.
$running = Get-Process shotpilot-app -ErrorAction SilentlyContinue
if ($running) {
    $running | Stop-Process -Force
    $running | Wait-Process -Timeout 10 -ErrorAction SilentlyContinue
    Write-Output "closed the running app ($($running.Count) window)"
}

if (-not $NoBuild) {
    $cargoArgs = @('build', '-p', 'shotpilot', '-p', 'shotpilot-app')
    if (-not $Dev) { $cargoArgs += '--release' }
    Push-Location $root
    try {
        $log = & cargo @cargoArgs 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    if ($code -ne 0) {
        # keep the errors, drop the "Compiling ..." lines
        $log | Where-Object { $_ -notmatch '^\s*(Compiling|Checking|Downloaded|Downloading|Locking|Updating|Adding)\b' } |
            Select-Object -Last 60 | Write-Output
        Write-Error "cargo build failed (exit $code)"
    }
    $warnings = @($log | Where-Object { $_ -match '^warning' }).Count
    $log | Where-Object { $_ -match '^\s*Finished' } | Write-Output
    if ($warnings -gt 0) { Write-Output "warnings: $warnings (cargo clippy --all-targets to see them)" }
}

if (-not (Test-Path $exe)) { Write-Error "no build at $exe (run without -NoBuild)" }

# Start from the user's home folder, so a shots.yaml in the current folder is not picked up by accident.
$start = @{ FilePath = $exe; WorkingDirectory = $env:USERPROFILE; PassThru = $true }
if ($Project) { $start.ArgumentList = @("`"$Project`"") }
$p = Start-Process @start
Start-Sleep -Seconds 3
if ($p.HasExited) {
    Write-Error "the app exited right after start (exit $($p.ExitCode)). Try -Dev to see its console."
}
$where = if ($Project) { $Project } else { 'home' }
Write-Output "launched $profileDir build (pid $($p.Id)) -> $where"
