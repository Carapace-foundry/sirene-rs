# ==============================================================================
# Builder stage
# ==============================================================================
FROM rust:1-slim-bookworm AS builder

WORKDIR /build

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary for sirene
RUN cargo build --release --bin sirene && \
    strip target/release/sirene

# ==============================================================================
# Runtime stage (minimal and secure)
# ==============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies (certificates, healthcheck tool)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root system user and group
RUN groupadd -g 1000 sirene && \
    useradd -u 1000 -g sirene -s /bin/false -m sirene

# Copy binary from builder
COPY --from=builder --chown=sirene:sirene /build/target/release/sirene /usr/local/bin/sirene

# Setup data directory for index volume
WORKDIR /data
RUN chown sirene:sirene /data

USER sirene

# Expose HTTP REST API microservice port
EXPOSE 8080

# Healthcheck
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["sirene"]
CMD ["serve", "--index", "/data/sirene.idx", "--host", "0.0.0.0", "--port", "8080"]
