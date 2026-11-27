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

.PHONY: all help check check-release docs release run demo demo-reset benchmark restart logs clean clean-docs clean-full geoip-check geoip-update release-dev restart-dev

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

PROFILE := release
BIN = target/$(PROFILE)/prieco_web

release: geoip-check ## Optimised build
	cargo build --release -p prieco_web --features $(FEATURES)

release-dev: geoip-check ## Faster-compiling build
	cargo build --profile release-dev -p prieco_web --features $(FEATURES)

run: release docs ## Start PriEco
	@ulimit -n 65535 || echo "⚠  could not raise open file limit"; \
	if [ -e "$(ORT_LIBS)/libonnxruntime.so" ]; then \
	  export ORT_DYLIB_PATH="$(ORT_LIBS)/libonnxruntime.so"; \
	  export LD_LIBRARY_PATH="$(ORT_LIBS)$(if $(CUDNN_LIBS),:$(CUDNN_LIBS))$${LD_LIBRARY_PATH:+:$$LD_LIBRARY_PATH}"; \
	fi; \
	exec ./$(BIN)

###
# Demo
###
DEMO := demo
DEMO_IMPORT := /tmp/res_imp
DEMO_PORT ?= 8484

demo: ## Sandbox, seeded with sample results
	cargo build --profile release-dev -p prieco_web
	mkdir -p $(DEMO)/runtime/config $(DEMO)/runtime/idx/{tantivy,meta,vectors,blob}
	ln -sfn ../frontend $(DEMO)/frontend
	ln -sfn ../backend $(DEMO)/backend
	sed -E 's/"port": [0-9]+/"port": $(DEMO_PORT)/' backend/demo/settings.conf > $(DEMO)/runtime/config/settings.conf
	[ -f $(DEMO)/runtime/.seeded ] || { python3 -I backend/demo/seed.py backend/demo/results.tsv $(DEMO_IMPORT)/demo.zip && touch $(DEMO)/runtime/.seeded; }
	@cd $(DEMO); \
	ulimit -n 65535 || echo "⚠  could not raise open file limit"; \
	if [ -e "$(ORT_LIBS)/libonnxruntime.so" ]; then \
	  export ORT_DYLIB_PATH="$(ORT_LIBS)/libonnxruntime.so"; \
	  export LD_LIBRARY_PATH="$(ORT_LIBS)$${LD_LIBRARY_PATH:+:$$LD_LIBRARY_PATH}"; \
	fi; \
	unset PRIECO_TICKET; \
	exec ../target/release-dev/prieco_web

demo-reset: ## Wipe the demo index
	rm -rf $(DEMO)

###
# Benchmark
###
benchmark: ## The running PriEco to run all benchmarks, or only SETS="trec_dl_2019 trec_dl_2020"
	@set -e; \
	systemctl --user is-active --quiet prieco || { echo "✗ prieco is not running, start it with: make restart"; exit 1; }; \
	bin=$$(systemctl --user show -p ExecStart --value prieco | sed -n 's/.*path=\([^ ;]*\).*/\1/p'); \
	grep -qa "Failed to listen for SIGUSR1" $$bin || { echo "✗ $$bin has no benchmark listener, run: make restart"; exit 1; }; \
	started=$$(date -d "$$(systemctl --user show -p ExecMainStartTimestamp --value prieco)" +%s); \
	if [ $$(stat -c %Y $$bin) -gt $$started ]; then \
	  echo "✗ prieco runs an older build than $$bin, run: make restart"; exit 1; \
	fi; \
	mkdir -p runtime/benchmarks; \
	printf '%s\n' $(SETS) > runtime/benchmarks/sets.txt; \
	systemctl --user kill --signal=SIGUSR1 --kill-whom=main prieco; \
	echo "📏 Benchmark started, follow it with: make logs"

###
# Server (systemd user service "prieco")
###
SERVICE := $(HOME)/.config/systemd/user/prieco.service

restart: release ## Build while the old version runs, deploy the service, restart
restart-dev: release-dev ## Same, with the faster-compiling release-dev build
restart-dev: PROFILE := release-dev
restart restart-dev: docs
	mkdir -p $(dir $(SERVICE))
	sed -e 's|@DIR@|$(CURDIR)|g' -e 's|@BIN@|$(CURDIR)/$(BIN)|g' \
	    -e 's|@ORT_LIBS@|$(ORT_LIBS)|g' -e 's|@LD_PATH@|$(ORT_LIBS)$(if $(CUDNN_LIBS),:$(CUDNN_LIBS))|g' \
	    container/prieco.service > $(SERVICE)
	loginctl show-user $(USER) -p Linger --value | grep -q yes || loginctl enable-linger
	systemctl --user daemon-reload
	systemctl --user enable prieco
	systemctl --user restart prieco

logs: ## Follow the service log
	journalctl --user -u prieco -f

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
