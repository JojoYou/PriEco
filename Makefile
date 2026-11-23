SHELL := /bin/bash
.DELETE_ON_ERROR:

export RUSTFLAGS := -C target-cpu=native
FEATURES ?= cuda

-include runtime/local.mk
ORT_LIBS ?= $(CURDIR)/runtime/libs
CUDNN_LIBS ?=

SITE := frontend/site
DOCS_STAMP := $(SITE)/.built
DOC_SRC := frontend/mkdocs.yml $(shell find frontend/docs backend/crates \
	-name '*.md' -o -name '*.rs' -o -name '*.css' -o -name 'Cargo.toml')

GEOIP := backend/crates/crawler/src/data/GeoLite2-Country.mmdb
GEOIP_MAX_DAYS := 30

.PHONY: all help check check-release docs release run benchmark restart logs clean clean-docs clean-full geoip-check geoip-update

all: check docs release

help:
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | sed 's/:.*## /\t/' | column -t -s $$'\t'

###
# Quality
###
check:
	cargo clippy --fix --allow-dirty --allow-no-vcs --allow-staged --all-targets -- -D warnings
	cargo fmt
	cargo deny check
	cargo test

check-release:
	cargo test --profile release-dev --features prieco_web/$(FEATURES)

###
# Build & run
###
docs: $(DOCS_STAMP) ## MkDocs + API docs into frontend/site

$(DOCS_STAMP): $(DOC_SRC)
	@if command -v mkdocs >/dev/null; then \
	  mkdocs build -q -f frontend/mkdocs.yml --site-dir $(CURDIR)/$(SITE); \
	else \
	  echo "⚠  mkdocs not installed, only API docs are built (pip install mkdocs-material)"; \
	fi
	cargo doc --workspace --no-deps --document-private-items
	mkdir -p $(SITE)
	rm -rf $(SITE)/api && cp -a target/doc $(SITE)/api
	touch $@

release: geoip-check ## Optimised build
	cargo build --release -p prieco_web --features $(FEATURES)

run: release docs ## Start PriEco
	@ulimit -n 65535 || echo "⚠  could not raise open file limit"; \
	if [ -e "$(ORT_LIBS)/libonnxruntime.so" ]; then \
	  export ORT_DYLIB_PATH="$(ORT_LIBS)/libonnxruntime.so"; \
	  export LD_LIBRARY_PATH="$(ORT_LIBS)$(if $(CUDNN_LIBS),:$(CUDNN_LIBS))$${LD_LIBRARY_PATH:+:$$LD_LIBRARY_PATH}"; \
	fi; \
	exec ./target/release/prieco_web

###
# Benchmark
###
benchmark: ## The running PriEco to run all benchmarks
	@set -e; \
	bin=target/release/prieco_web; \
	systemctl is-active --quiet prieco || { echo "✗ prieco is not running, start it with: make restart"; exit 1; }; \
	grep -qa "Failed to listen for SIGUSR1" $$bin || { echo "✗ $$bin has no benchmark listener, run: make restart"; exit 1; }; \
	started=$$(date -d "$$(systemctl show -p ExecMainStartTimestamp --value prieco)" +%s); \
	if [ $$(stat -c %Y $$bin) -gt $$started ]; then \
	  echo "✗ prieco runs an older build than $$bin, run: make restart"; exit 1; \
	fi; \
	systemctl kill --signal=SIGUSR1 --kill-whom=main prieco; \
	echo "📏 Benchmark started, follow it with: make logs"

###
# Server (systemd service "prieco")
###
restart: release docs ## Build while the old version runs
	systemctl restart prieco

logs: ## Follow the service log
	journalctl -u prieco -f

###
# Cleanup
###
clean-docs:
	rm -rf $(SITE)

clean: clean-docs
	cargo clean

clean-full: clean
	rm -rf /tmp/prieco_*
	@read -p "Also delete runtime/ (index, settings, iroh identity)? [y/N] " a; \
	if [ "$$a" = y ]; then rm -rf runtime; else echo "runtime/ kept"; fi

###
# GeoLite2
###
geoip-check:
	@ts=$$(git log -1 --format=%ct -- $(GEOIP) 2>/dev/null); \
	[ -n "$$ts" ] || ts=$$(stat -c %Y $(GEOIP)); \
	age=$$(( ($$(date +%s) - ts) / 86400 )); \
	if [ $$age -gt $(GEOIP_MAX_DAYS) ]; then \
	  echo "⚠  GeoLite2 is $$age days old, run: make geoip-update"; \
	fi

geoip-update: ## Download the latest GeoLite2
	@set -eo pipefail; \
	. backend/.env; \
	: "$${MAXMIND_ACCOUNT_ID:?missing in backend/.env}" "$${MAXMIND_LICENSE_KEY:?missing in backend/.env}"; \
	tmp=$$(mktemp -d); trap 'rm -rf "$$tmp"' EXIT; \
	curl -fsSL -u "$$MAXMIND_ACCOUNT_ID:$$MAXMIND_LICENSE_KEY" \
	  "https://download.maxmind.com/geoip/databases/GeoLite2-Country/download?suffix=tar.gz" \
	  | tar -xz -C "$$tmp"; \
	mv "$$tmp"/*/GeoLite2-Country.mmdb $(GEOIP); \
	echo "GeoLite2 updated, rebuild with: make release"
