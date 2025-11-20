FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main
RUN apt-get update && apt-get install -y clang-14 libclang-14-dev libstdc++-12-dev
