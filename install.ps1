# kazma installer
#   iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
# Installs kz.exe to %LOCALAPPDATA%\Programs\kazma and adds it to your user PATH.
# No admin rights needed. Set $env:KAZMA_VERSION = "v0.1.0" to pin a version.
$ErrorActionPreference = 'Stop'
$repo = 'hhasanguclu/kazma'
$dir  = Join-Path $env:LOCALAPPDATA 'Programs\kazma'
$exe  = Join-Path $dir 'kz.exe'
$url  = if ($env:KAZMA_VERSION) { "https://github.com/$repo/releases/download/$($env:KAZMA_VERSION)/kz.exe" }
        else { "https://github.com/$repo/releases/latest/download/kz.exe" }

Write-Host "  kazma :: installing to $dir" -ForegroundColor Green
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$tmp = "$exe.download"
Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $tmp
Move-Item -Force $tmp $exe
Unblock-File $exe -ErrorAction SilentlyContinue

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$parts = @($userPath -split ';' | Where-Object { $_ })
if (-not ($parts | Where-Object { $_.TrimEnd('\') -ieq $dir })) {
    [Environment]::SetEnvironmentVariable('Path', (($parts + $dir) -join ';'), 'User')
    Write-Host "  [ok] added to PATH (open a new terminal)" -ForegroundColor Green
}
if (-not (($env:Path -split ';') | Where-Object { $_.TrimEnd('\') -ieq $dir })) { $env:Path += ";$dir" }

& $exe --version
Write-Host "  [ok] done. run: kz" -ForegroundColor Green
