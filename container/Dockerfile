FROM rust:1.96-bookworm AS builder

# Dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang llvm libclang-dev cmake \
    libssl-dev libzstd-dev \
    libsnappy-dev liblz4-dev \
    python3 git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

ARG FEATURES=cuda

# Cache
COPY Cargo.toml Cargo.lock ./
COPY backend/crates/core/Cargo.toml       backend/crates/core/Cargo.toml
COPY backend/crates/blob/Cargo.toml       backend/crates/blob/Cargo.toml
COPY backend/crates/pagerank/Cargo.toml   backend/crates/pagerank/Cargo.toml
COPY backend/crates/insert/Cargo.toml     backend/crates/insert/Cargo.toml
COPY backend/crates/web/Cargo.toml        backend/crates/web/Cargo.toml
COPY backend/crates/crawler/Cargo.toml    backend/crates/crawler/Cargo.toml
COPY backend/crates/benchmark/Cargo.toml  backend/crates/benchmark/Cargo.toml
COPY backend/vendor/ backend/vendor/

RUN for crate in core blob pagerank insert crawler benchmark; do \
    mkdir -p backend/crates/$crate/src && echo "pub fn stub() {}" > backend/crates/$crate/src/lib.rs; \
    done && \
    mkdir -p backend/crates/web/src && echo "fn main() {}" > backend/crates/web/src/main.rs

RUN cargo build --release -p prieco_web --features ${FEATURES} 2>/dev/null || true
RUN find target/release -name "*.d" -delete

# Sources
COPY backend/ backend/

RUN cargo build --release -p prieco_web --features ${FEATURES}

# Runtime
FROM nvidia/cuda:11.8.0-cudnn8-runtime-ubuntu22.04 AS runtime

# Install libraries
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libzstd1 libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1001 prieco

WORKDIR /app

RUN chown prieco:prieco /app

COPY --chown=prieco:prieco --from=builder /build/target/release/prieco_web ./prieco_web
COPY --chown=prieco:prieco --from=builder /build/target/release/libonnxruntime*.so* /usr/local/lib/
RUN ldconfig

ENV ORT_STRATEGY=system
ENV ORT_DYLIB_PATH=/usr/local/lib/libonnxruntime.so

COPY --chown=prieco:prieco backend/data/bge/model.onnx ./backend/data/bge/model.onnx
COPY --chown=prieco:prieco backend/data/paraphrase-multilingual-MiniLM-L12-v2_O3.onnx ./backend/data/paraphrase-multilingual-MiniLM-L12-v2_O3.onnx
COPY --chown=prieco:prieco frontend/templates/ ./frontend/templates/
COPY --chown=prieco:prieco frontend/static/    ./frontend/static/

RUN mkdir -p /app/data/tantivy /app/data/blobs /app/data/meta /app/data/vectors \
             /app/runtime/config /app/frontend/site \
    && chown -R prieco:prieco /app/data /app/runtime /app/frontend/site

VOLUME [ \
  "/app/data/tantivy", "/app/data/blobs", "/app/data/meta", "/app/data/vectors", \
  "/app/runtime" \
]

COPY container/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

USER prieco
ENTRYPOINT ["/app/entrypoint.sh"]
