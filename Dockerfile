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
COPY data/ivf/centroids.bin  data/ivf/centroids.bin
COPY data/bge/model.onnx     data/bge/model.onnx
COPY data/bge/tokenizer.json data/bge/tokenizer.json
COPY data/domains.txt        data/domains.txt
COPY data/tokenizer.json     data/tokenizer.json
COPY data/paraphrase-multilingual-MiniLM-L12-v2_O3.onnx    data/paraphrase-multilingual-MiniLM-L12-v2_O3.onnx

# Sources
COPY crates/ crates/
COPY .env    .env

RUN cargo build --release

# Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libzstd1 libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/prieco_web ./prieco_web

# Libs
COPY libs/ /usr/local/lib/
RUN ln -sf /usr/local/lib/libonnxruntime.so.1.16.0 /usr/local/lib/libonnxruntime.so.1.16.3 && \
    ln -sf /usr/local/lib/libonnxruntime.so.1.16.3 /usr/local/lib/libonnxruntime.so && \
    ldconfig

# Assets
COPY templates/ ./templates/
COPY static/    ./static/

RUN useradd -m -u 1001 prieco
USER prieco

ENTRYPOINT ["./prieco_web"]

# Build
# docker build -t prieco:latest .
# Run (example)
# docker run -it -p 8088:8080   -v ./prieco_data:/app/data   -u root   --name prieco   prieco:latest
