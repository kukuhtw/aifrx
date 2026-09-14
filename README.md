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

See the [Telegram command reference](docs/telegram-command-reference.md), [platform subscription fees vs. MT5 and broker costs](docs/subscription-fees-vs-mt5-and-broker-costs.md), the [pricing, billing, and payment administration guide](docs/pricing-billing-and-payment-administration.md), [application user guide and safety FAQ](docs/how-to-use-the-application.md), [SaaS, administration, AI, market data, and API ownership guide](docs/saas-administration-ai-and-market-data.md), [product overview](docs/product-overview.md), [MT5 and the Rust–Python architecture](docs/mt5-and-dual-tech-stack.md), [user journey and data flow](docs/user-journey-and-data-flow.md), [API documentation](docs/api.md), [architecture](docs/architecture.md), [security](docs/security.md), and [risk controls](docs/risk-controls.md).

## Admin dashboard

A protected read-only operations dashboard is available at `/admin/`. Configure it with `ADMIN_USERNAME` and `ADMIN_PASSWORD_HASH_B64`; see the [admin dashboard guide](docs/admin-dashboard.md).

## Author

**Kukuh TW** — Creator and lead developer

- LinkedIn: [linkedin.com/in/kukuhtw](https://www.linkedin.com/in/kukuhtw)
- Email: [kukuhtw@gmail.com](mailto:kukuhtw@gmail.com)

Additional author information is available in [AUTHORS.md](AUTHORS.md).
