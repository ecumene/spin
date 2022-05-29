FROM rust:1.61-slim-buster

ENV DEBIAN_FRONTEND=noninteractive
RUN apt update && apt install -y wget git pkg-config libssl-dev make && rm -rf /var/lib/apt/lists/*

WORKDIR /tmp
RUN wget https://github.com/WebAssembly/binaryen/releases/download/version_105/binaryen-version_105-x86_64-linux.tar.gz
RUN tar -zxvf binaryen-version_105-x86_64-linux.tar.gz
RUN ls -la /tmp/binaryen-version_105/bin
RUN cp /tmp/binaryen-version_105/bin/* /usr/local/bin/
RUN rm -rf /tmp/binaryen-version_105


WORKDIR /usr/src/spin
RUN rustup target add wasm32-wasi
COPY . .


RUN make build
RUN cp target/release/spin /usr/local/bin/spin
WORKDIR /app
RUN rm -rf /usr/src/spin
ENV RUST_LOG=spin=trace

ENTRYPOINT [ "spin" ]