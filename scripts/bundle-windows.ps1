param([switch]$SkipSetup)
$ErrorActionPreference = 'Stop'
Set-Location "$PSScriptRoot/.."
if (!$SkipSetup) { . "$PSScriptRoot/setup-windows.ps1" }
function Run-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}
Run-Checked python @('scripts/bundle-windows.py')
Push-Location frontend
try {
    Run-Checked bun @('install', '--frozen-lockfile')
    Run-Checked bun @('run', 'build')
} finally { Pop-Location }
Push-Location desktop
try {
    Run-Checked bun @('../frontend/node_modules/@tauri-apps/cli/tauri.js', 'build', '--bundles', 'nsis', '--config', 'native/windows-resources.json')
    Run-Checked cargo @('build', '--locked', '--release', '--example', 'bundled_media')
} finally { Pop-Location }
Run-Checked python @('scripts/check-windows-bundle.py')
