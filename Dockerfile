FROM rust:1.61 as builder
WORKDIR /app
COPY . .
RUN rustup target add wasm32-wasi
RUN cargo build --all-features --release

FROM  alpine
COPY --from=builder /app/target/release/spin /app/spin
WORKDIR /app
CMD ["/app/spin", "up"]

