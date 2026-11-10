# Packages the already-built release executable into a portable zip.
# Run `npm run tauri:build` first.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$version = (Get-Content (Join-Path $root "package.json") -Raw | ConvertFrom-Json).version
$exe = Join-Path $root "src-tauri\target\release\portwatch.exe"
if (-not (Test-Path $exe)) { throw "portwatch.exe not found. Run 'npm run tauri:build' first." }

$name = "PortWatch-$version-x64-portable"
$out = Join-Path $root "dist-portable"
$stage = Join-Path $out $name
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Path $stage -Force | Out-Null

Copy-Item $exe (Join-Path $stage "PortWatch.exe")
Copy-Item (Join-Path $root "LICENSE") $stage
Copy-Item (Join-Path $root "README.md") $stage
# The marker file switches PortWatch to storing data in .\data next to the exe.
Set-Content -Path (Join-Path $stage "portwatch.portable") -Value "Delete this file to store data in %APPDATA% instead." -Encoding ASCII

$zip = Join-Path $out "$name.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path $stage -DestinationPath $zip
Write-Host "Created $zip"
