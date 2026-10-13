# Build container for ms900ctl.
#
#   docker build -t ms900ctl-build .
#   docker run --rm -v "$PWD/dist:/out" ms900ctl-build
#
# produces dist/ms900ctl (dynamically linked against glibc, libudev is
# linked statically through hidapi's bundled sources).  You still need the
# udev rule from packaging/70-ms900.rules on the host.
FROM rust:1-bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libudev-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev \
        libx11-dev libxcursor-dev libxi-dev libxrandr-dev libfontconfig1-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
RUN cargo build --release --locked && cargo test --release --lib --locked

FROM debian:bookworm-slim
COPY --from=build /src/target/release/ms900ctl /ms900ctl
COPY packaging/70-ms900.rules /70-ms900.rules
CMD ["sh", "-c", "cp /ms900ctl /70-ms900.rules /out/ && echo 'copied to /out'"]
