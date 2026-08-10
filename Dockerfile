# Build stage
FROM rust:1-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata wget \
    && rm -rf /var/lib/apt/lists/*

# Static FFmpeg
COPY --from=mwader/static-ffmpeg:6.1.1 /ffmpeg /usr/local/bin/ffmpeg

RUN groupadd -g 1000 appgroup \
    && useradd -u 1000 -g appgroup -m appuser

WORKDIR /app
COPY --from=builder /build/target/release/bilibili-downloader-server /app/bilibili-downloader-server
RUN chown -R appuser:appgroup /app

USER appuser
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/api/v1/health || exit 1

ENTRYPOINT ["/app/bilibili-downloader-server"]
