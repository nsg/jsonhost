# --- Build a fully static binary against musl ---
FROM rust:1.96-alpine AS builder
RUN apk add --no-cache musl-dev gcc ca-certificates
WORKDIR /app
COPY . .
RUN cargo build --release

# --- Minimal single-binary image ---
FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/target/release/jsonhost /jsonhost
EXPOSE 8090
ENTRYPOINT ["/jsonhost"]
