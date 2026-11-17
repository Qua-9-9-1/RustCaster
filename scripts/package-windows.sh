#!/bin/sh
# Builds the release .exe and gathers everything needed to play into rustcaster/dist/rustcaster-windows-x64/.
# Runs inside the build image (see Dockerfile), with the repo mounted and rustcaster/ as working dir:
#   docker run --rm -v "<repo>:/repo" -w /repo/rustcaster rustcaster-build sh ../scripts/package-windows.sh
set -eu

target=x86_64-pc-windows-gnu
out=dist/rustcaster-windows-x64

cargo build --release --target "$target"

rm -rf "$out"
mkdir -p "$out"
cp "target/$target/release/rustcaster.exe" "$out/"
cp /opt/sdl2/x86_64-w64-mingw32/bin/SDL2.dll "$out/"
cp -r assets maps "$out/"

echo "Packaged into rustcaster/$out"
