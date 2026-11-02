SHELL := /bin/bash
.DEFAULT_GOAL := help

COMPOSE := docker compose --env-file .env -f infrastructure/docker/compose.yml
API := $(COMPOSE) exec -T api
# frontend roda num container node:22 (não depende do Node local)
NODE_IMAGE := node:22-alpine
NODE := docker run --rm -t -v "$(CURDIR)/apps/desktop":/app -v nucleo-npm-cache:/home/node/.npm \
	-w /app -u $$(id -u):$$(id -g) -e npm_config_cache=/home/node/.npm $(NODE_IMAGE)

.PHONY: help env up down logs ps wait test lint fmt smoke seed reset shell psql migrate \
	web-install web-lint web-build web-dev

help: ## lista os comandos
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

env:
	@test -f .env || (cp .env.example .env && echo ".env criado a partir de .env.example")

up: env ## sobe db, storage e api; espera a api ficar saudável
	$(COMPOSE) up -d --build --wait --wait-timeout 180
	@$(MAKE) --no-print-directory ps

down: ## para os containers (mantém volumes)
	$(COMPOSE) down

logs: ## segue os logs (ex.: make logs s=api)
	$(COMPOSE) logs -f --tail=200 $(s)

ps:
	@$(COMPOSE) ps

test: ## pytest dentro do container da api (banco nucleo_test)
	$(API) pytest $(args)

lint: ## ruff + mypy no backend; tsc + eslint no desktop
	$(API) ruff check .
	$(API) ruff format --check .
	$(API) mypy app
	@$(MAKE) --no-print-directory web-lint

fmt: ## formata o backend
	$(API) ruff format .
	$(API) ruff check --fix .

smoke: ## smoke test ponta a ponta via HTTP
	./scripts/smoke.sh

seed: ## popula duas empresas fictícias
	$(API) python - < scripts/seed.py

reset: env ## APAGA volumes e recria o ambiente do zero
	$(COMPOSE) down -v --remove-orphans
	@$(MAKE) --no-print-directory up

migrate: ## roda alembic upgrade head
	$(API) alembic upgrade head

shell: ## shell no container da api
	$(COMPOSE) exec api bash

psql: ## psql como superusuário
	$(COMPOSE) exec db psql -U postgres -d nucleo

web-install: ## npm ci do desktop (container node:22)
	$(NODE) sh -c 'test -d node_modules || npm ci'

web-lint: web-install ## typecheck + eslint do desktop
	$(NODE) npm run lint

web-build: web-install ## build de produção do frontend
	$(NODE) npm run build

web-dev: ## frontend no navegador (requer Node 20+ local): http://localhost:1420
	cd apps/desktop && npm install && npm run dev
