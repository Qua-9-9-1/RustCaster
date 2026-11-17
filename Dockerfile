# Build image: cross-compiles the game into a Windows .exe (x86_64-pc-windows-gnu)
FROM rust:1-slim

ARG SDL_VERSION=2.30.9

RUN apt-get update \
    && apt-get install -y --no-install-recommends gcc-mingw-w64-x86-64 curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL "https://github.com/libsdl-org/SDL/releases/download/release-${SDL_VERSION}/SDL2-devel-${SDL_VERSION}-mingw.tar.gz" \
    | tar xz -C /opt \
    && mv "/opt/SDL2-${SDL_VERSION}" /opt/sdl2

RUN rustup target add x86_64-pc-windows-gnu

ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-L native=/opt/sdl2/x86_64-w64-mingw32/lib"

WORKDIR /app
CMD cargo build --target x86_64-pc-windows-gnu \
    && cp /opt/sdl2/x86_64-w64-mingw32/bin/SDL2.dll target/x86_64-pc-windows-gnu/debug/
