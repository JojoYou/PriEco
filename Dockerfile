FROM rust:1.96-bookworm AS builder

# Dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang llvm libclang-dev cmake \
    libssl-dev libzstd-dev \
    libsnappy-dev liblz4-dev \
    python3 git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache
COPY Cargo.toml Cargo.lock ./
COPY crates/core/Cargo.toml      crates/core/Cargo.toml
COPY crates/blob/Cargo.toml      crates/blob/Cargo.toml
COPY crates/pagerank/Cargo.toml  crates/pagerank/Cargo.toml
COPY crates/insert/Cargo.toml    crates/insert/Cargo.toml
COPY crates/web/Cargo.toml       crates/web/Cargo.toml
COPY crates/crawler/Cargo.toml   crates/crawler/Cargo.toml
COPY vendor/ vendor/

RUN for crate in core blob pagerank insert crawler; do \
    mkdir -p crates/$crate/src && echo "pub fn stub() {}" > crates/$crate/src/lib.rs; \
    done && \
    mkdir -p crates/web/src && echo "fn main() {}" > crates/web/src/main.rs

RUN cargo build --release 2>/dev/null || true
RUN find target/release -name "*.d" -delete

# Data files
COPY data/ data/

# Sources
COPY crates/ crates/

RUN cargo build --release

# Runtime
FROM nvidia/cuda:11.8.0-cudnn8-runtime-ubuntu22.04 AS runtime

# Install required system libraries
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libzstd1 libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1001 prieco

WORKDIR /app

RUN chown -R prieco:prieco /app

COPY --chown=prieco:prieco --from=builder /build/target/release/prieco_web ./prieco_web
COPY --chown=prieco:prieco --from=builder /build/target/release/libonnxruntime*.so* /usr/local/lib/
RUN ldconfig

ENV ORT_STRATEGY=system
ENV ORT_DYLIB_PATH=/usr/local/lib/libonnxruntime.so

COPY --chown=prieco:prieco data/      ./data/
COPY --chown=prieco:prieco templates/ ./templates/
COPY --chown=prieco:prieco static/    ./static/


RUN mkdir -p /app/data/tantivy /app/data/blobs /app/data/meta /app/data/vectors \
             /app/config /app/models \
    && touch /app/GeoLite2-Country.mmdb \
    && chown -R prieco:prieco /app/data /app/config /app/models /app/GeoLite2-Country.mmdb

VOLUME [ \
  "/app/data/tantivy", "/app/data/blobs", "/app/data/meta", "/app/data/vectors", \
  "/app/config", "/app/models", "/app/GeoLite2-Country.mmdb" \
]

ENTRYPOINT ["/app/entrypoint.sh"]



# Build
# docker build -t prieco:latest .
# Run (example)
# docker run -it -p 8088:8080   -v ./prieco_data:/app/data   -u root   --name prieco   prieco:latest
