FROM --platform=$BUILDPLATFORM rust:1.60 as builder
WORKDIR /app
ADD . /app
RUN rustup target add wasm32-wasi
RUN rustup target add aarch64-unknown-linux-gnu
RUN cargo build --all-features --target aarch64-unknown-linux-gnu --release

FROM --platform=$BUILDPLATFORM alpine
COPY --from=builder target/aarch64-unknown-linux-gnu/release/spin /app/spin
WORKDIR /app
CMD ["/app/spin", "up"]

