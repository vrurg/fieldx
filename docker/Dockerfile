FROM buildpack-deps:bookworm AS fx-rust-base

ARG RUST_VERSION

WORKDIR /rust

ENV RUSTUP_HOME=/rust/rustup \
    CARGO_HOME=/rust/cargo \
    PATH=/rust/cargo/bin:$PATH

RUN apt-get update && \
    apt-get install -y rsync && \
    rm -rf /var/lib/apt/lists/*

RUN (wget -O - https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain ${RUST_VERSION}) \
    && rustup default ${RUST_VERSION} \
    && cargo +stable install cargo-make

FROM fx-rust-base AS fieldx-make

WORKDIR /fieldx
COPY . .
COPY ./docker/scripts/execute.sh /usr/local/bin/execute.sh
RUN chmod +x /usr/local/bin/execute.sh

ENTRYPOINT [ "/usr/local/bin/execute.sh" ]