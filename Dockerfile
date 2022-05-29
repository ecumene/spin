FROM rust:1.61 as builder
ADD . .
RUN rustup target add wasm32-wasi
RUN cargo build --all-features --release

FROM  alpine
COPY --from=builder target/release/spin /app/spin
WORKDIR /app
CMD ["/app/spin", "up"]

