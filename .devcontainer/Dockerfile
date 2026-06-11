FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

RUN apt-get update && apt-get install -y \
    pkg-config \
    libasound2-dev \
    libudev-dev \
    libxkbcommon-dev \
    libwayland-dev \
    libx11-dev \
    libxrandr-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add clippy rustfmt
