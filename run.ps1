# Build the game in Docker (cross-compiled to Windows) then run it natively.
# Usage: .\run.ps1                    (runs maps\data.txt)
#        .\run.ps1 maps\map1.txt      (map path relative to rustcaster/)
#        .\run.ps1 -Build             (build only)

param(
    [string]$Map = "maps\data.txt",
    [switch]$Build
)

$ErrorActionPreference = "Stop"

$root = $PSScriptRoot
$project = Join-Path $root "rustcaster"

docker build -q -t rustcaster-build $root | Out-Null
if ($LASTEXITCODE -ne 0) { throw "docker image build failed" }

# named volume keeps the crates.io cache between builds
docker run --rm -v "${project}:/app" -v rustcaster-cargo-registry:/usr/local/cargo/registry rustcaster-build
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

if ($Build) { exit 0 }

Push-Location $project
try {
    & ".\target\x86_64-pc-windows-gnu\debug\rustcaster.exe" $Map
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
