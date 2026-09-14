# AI Forex Trading Assistant

Demo-first, human-confirmed forex decision support. Rust owns authorization, risk checks, idempotency and persistence; the private Python service is only an MT5 adapter. Funds remain at the user's broker. The app cannot withdraw funds and does not guarantee profit.

## Current vertical slice

- Fetch a fresh quote, request structured OpenAI analysis, validate it, and persist it.
- Create a non-executing trade intent.
- Explicitly confirm with an idempotency key; re-fetch price, enforce permissions, demo/live gates, limits and slippage, then execute.
- AES-256-GCM credential utility, multi-user ownership enforcement, audit events, and kill switch.
- Mock bridge by default. The native `MetaTrader5` package and terminal require a Windows deployment; see [MT5 deployment](docs/mt5-deployment.md).

Telegram command handlers, account-connection endpoints, positions/history and modification flows remain subsequent product slices; the HTTP domain boundary is implemented first so those thin adapters cannot bypass controls.

## Run locally

1. Copy `.env.example` to `.env`, set secrets, and generate `ENCRYPTION_KEY` with `openssl rand -base64 32`.
2. Run `docker compose up --build`.
3. Check `GET http://localhost:8080/health`.

Live orders require all three explicit gates: `TRADING_MODE=LIVE`, global `LIVE_TRADING_ENABLED=true`, and the user's `live_trading_enabled=true`. Accounts also require verified, active and `TRADING_ENABLED` state. Defaults cannot execute live trades.

## API

- `POST /api/v1/analyses`
- `POST /api/v1/trade-intents`
- `POST /api/v1/trade-intents/{id}/confirm`
- `POST /api/v1/trading/stop`

See [API documentation](docs/api.md), [architecture](docs/architecture.md), [security](docs/security.md), and [risk controls](docs/risk-controls.md).

