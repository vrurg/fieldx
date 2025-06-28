FROM buildpack-deps:bookworm AS fx-rust-base

ARG RUST_VERSION

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    TMPDIR=/usr/local/rustup/tmp \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update && \
    apt-get install -y rsync && \
    rm -rf /var/lib/apt/lists/*

RUN mkdir -p /usr/local/rustup/tmp

WORKDIR /usr/local/rustup/tmp

RUN (wget -O - https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain ${RUST_VERSION}) \
    && rustup default ${RUST_VERSION} \
    && cargo +stable install cargo-make

FROM fx-rust-base AS fieldx-make

WORKDIR /fieldx
COPY . .
COPY ./docker/scripts/execute.sh /usr/local/bin/execute.sh
RUN chmod +x /usr/local/bin/execute.sh

ENTRYPOINT [ "/usr/local/bin/execute.sh" ]