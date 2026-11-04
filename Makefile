SHELL := /bin/bash
.DEFAULT_GOAL := help

# Ferramentas locais: Rust (rustup) e Node >= 18. Não há servidor, banco externo nem Docker.
# rustup instala em ~/.cargo/bin; exporta para os filhos (o `tauri dev` procura o cargo no PATH)
export PATH := $(HOME)/.cargo/bin:$(PATH)
CARGO := cargo
DESKTOP := apps/desktop
# pasta de dados usada em desenvolvimento (o app instalado usa a pasta de dados do SO)
DEV_DATA ?= $(CURDIR)/.dev-data

.PHONY: help install test lint fmt check seed reset-data dev build web-build manual

help: ## lista os comandos
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

install: ## instala dependências do frontend
	cd $(DESKTOP) && npm ci

test: ## testes do núcleo (SQLite em pasta temporária)
	$(CARGO) test -p nucleo-core $(args)

lint: ## fmt + clippy (núcleo e app) + tsc + eslint
	$(CARGO) fmt --all --check
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	cd $(DESKTOP) && npm run lint

fmt: ## formata o código Rust
	$(CARGO) fmt --all

check: lint test ## tudo que precisa estar verde antes de um commit

seed: ## cria empresa de demonstração em .dev-data (ex.: make seed empresa=techparts)
	$(CARGO) run -q -p nucleo-core --bin seed -- $(DEV_DATA) $(empresa)

reset-data: ## APAGA os dados de desenvolvimento (.dev-data)
	rm -rf $(DEV_DATA)

# WSL não expõe a GPU ao WebKitGTK (avisos libEGL/ZINK, janela em branco): renderiza sem DMA-BUF
dev: ## abre o app em modo desenvolvimento usando .dev-data
	cd $(DESKTOP) && NUCLEO_DATA_DIR=$(DEV_DATA) WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run tauri dev

build: ## gera o instalador da plataforma atual (Windows: .msi/.exe; Linux: .deb/.AppImage)
	cd $(DESKTOP) && npm run tauri build

web-build: ## só o build do frontend
	cd $(DESKTOP) && npm run build

# Chrome do Windows (WSL); em outro ambiente: make manual CHROME=/caminho/do/chrome
CHROME ?= /mnt/c/Program Files/Google/Chrome/Application/chrome.exe
MANUAL_PDF := docs/Nucleo-ERP-Documentacao.pdf

manual: ## gera o PDF da documentação master a partir de docs/manual/manual.html
	"$(CHROME)" --headless=new --disable-gpu --no-pdf-header-footer \
		--print-to-pdf="$$(wslpath -w $(CURDIR)/$(MANUAL_PDF))" \
		"file:///$$(wslpath -w $(CURDIR)/docs/manual/manual.html | tr '\\' '/')"
	@echo "PDF: $(MANUAL_PDF)"
