# kazma uninstaller (same as `kz remove`)
#   iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/uninstall.ps1 | iex
$ErrorActionPreference = 'Stop'
$dir  = Join-Path $env:LOCALAPPDATA 'Programs\kazma'
$data = Join-Path $env:LOCALAPPDATA 'kazma'

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$kept = @($userPath -split ';' | Where-Object { $_ -and ($_.TrimEnd('\') -ine $dir) })
[Environment]::SetEnvironmentVariable('Path', ($kept -join ';'), 'User')
Write-Host "  [ok] PATH entry removed" -ForegroundColor Green

foreach ($p in @($dir, $data)) {
    if (Test-Path $p) { Remove-Item -Recurse -Force $p; Write-Host "  [ok] removed $p" -ForegroundColor Green }
}
Write-Host "  kazma uninstalled." -ForegroundColor Green
