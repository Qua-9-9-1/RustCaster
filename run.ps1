# Build the game in Docker (cross-compiled to Windows) then run it natively.
# Usage: .\run.ps1                    (opens the menu)
#        .\run.ps1 maps\map1.txt      (plays a map directly, path relative to rustcaster/)
#        .\run.ps1 -Build             (build only)

param(
    [string]$Map = "",
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
    # the game finds assets/ and maps/ from the current dir (rustcaster/) or from the exe location
    $exe = ".\target\x86_64-pc-windows-gnu\debug\rustcaster.exe"
    if ($Map) { & $exe $Map } else { & $exe }
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
