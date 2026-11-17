# Build the game in Docker (cross-compiled to Windows) then run it natively.
# Usage: .\run.ps1                    (opens the menu)
#        .\run.ps1 maps\map1.txt      (plays a map directly, path relative to rustcaster/)
#        .\run.ps1 -Build             (build only)
#        .\run.ps1 -Package           (release build zipped into rustcaster\dist\rustcaster-windows-x64.zip)

param(
    [string]$Map = "",
    [switch]$Build,
    [switch]$Package
)

$ErrorActionPreference = "Stop"

$root = $PSScriptRoot
$project = Join-Path $root "rustcaster"

docker build -q -t rustcaster-build $root | Out-Null
if ($LASTEXITCODE -ne 0) { throw "docker image build failed" }

if ($Package) {
    docker run --rm -v "${root}:/repo" -v rustcaster-cargo-registry:/usr/local/cargo/registry -w /repo/rustcaster rustcaster-build sh ../scripts/package-windows.sh
    if ($LASTEXITCODE -ne 0) { throw "packaging failed" }
    $dist = Join-Path $project "dist"
    $zip = Join-Path $dist "rustcaster-windows-x64.zip"
    Compress-Archive -Path (Join-Path $dist "rustcaster-windows-x64") -DestinationPath $zip -Force
    Write-Host "Created $zip"
    exit 0
}

# named volume keeps the crates.io cache between builds
docker run --rm -v "${project}:/app" -v rustcaster-cargo-registry:/usr/local/cargo/registry rustcaster-build
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

if ($Build) { exit 0 }

Push-Location $project
try {
    # the game finds assets/ and maps/ from the current dir (rustcaster/) or from the exe location
    $exe = ".\target\x86_64-pc-windows-gnu\debug\rustcaster.exe"
    if ($Map) { & $exe $Map } else { & $exe }
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
