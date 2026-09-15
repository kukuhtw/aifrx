# Project Status Report

## 1. Purpose and How to Read This

This is a snapshot — **as of commit `d63446f`, 15 September 2026** — of what actually exists in this repository versus what is still planned. It exists so anyone (the project owner, a contributor, a reviewer) can answer "where do things actually stand?" in a few minutes, without cross-referencing a dozen documents.

This report is a **summary view**. For exhaustive, requirement-by-requirement status (with acceptance criteria and BRD traceability), see the [PRD's functional requirements](prd.md#7-functional-requirements) and [release phases](prd.md#11-release-phases) — this document distills those into one readable status page and adds the documentation work this report itself is part of.

Status markers used below:

- ✅ **Done** — implemented and working in the current codebase
- 🟡 **Partial** — a real foundation exists, but the feature isn't complete
- ⬜ **Not started** — no code exists for this yet

## 2. At a Glance

| Track | Status |
|---|---|
| Core trading workflow (analysis → intent → confirmation → mock execution) | ✅ Done |
| Risk engine (core checks) | ✅ Done |
| Data model & persistence (trading + billing schema) | ✅ Done |
| Security foundations (credential encryption, internal auth, audit log) | ✅ Done |
| Admin dashboard | 🟡 Partial — read-only console works; RBAC/MFA/mutations don't exist |
| Documentation | ✅ Done for the current foundation; grows as features ship |
| Licensing | ✅ Done |
| Telegram bot (the actual user-facing product) | ⬜ Not started |
| MT5 account onboarding / connection API | ⬜ Not started |
| Positions, history, close/modify | ⬜ Not started |
| Real MT5 connectivity (a live broker account) | ⬜ Not started — requires a Windows host, see [§5](#5-not-started) |
| Payment/billing (checkout, webhooks, reconciliation) | ⬜ Not started — schema only |
| Automated test coverage | 🟡 Partial — a handful of unit tests, no integration suite |
| Legal / regulatory / compliance review | ⬜ Not started |

**Bottom line:** the repository is a working **backend foundation and mock-trading vertical slice** — the core safety architecture (human confirmation, risk gates, idempotency, audit trail) is real and functional end-to-end against mock data. It is not yet a product a real user could open in Telegram and use; there is no Telegram interface, no real MT5 connectivity, and no billing.

## 3. Completed

### 3.1 Core backend and trading workflow

- Rust/Axum HTTP backend, running and health-checked (`GET /health`)
- `POST /api/v1/analyses` — fetches a quote, sends sanitized fields to OpenAI (or returns a controlled mock `WAIT` if no API key is configured), validates and persists the structured result
- `POST /api/v1/trade-intents` — creates a non-executing intent; never places an order
- `POST /api/v1/trade-intents/{id}/confirm` — idempotency-key-protected confirmation that re-fetches price, re-validates ownership/permissions/risk/slippage, then executes
- `POST /api/v1/trading/stop` — kill switch; blocks new BUY/SELL while leaving analysis and position-closing untouched (closing itself isn't implemented yet — see [§5](#5-not-started))
- Market-data freshness validation (`MARKET_DATA_MAX_AGE_SECONDS`)
- Transactional idempotency (unique hashed key, row locking) preventing duplicate orders

### 3.2 Risk engine (core checks)

- Account ownership, active/verified state, and permission-mode (`READ_ONLY`/`TRADING_ENABLED`) checks
- Lot-size, open-position-count, and trades-per-day limits
- Stop-loss/take-profit direction validation
- Slippage check against a freshly re-fetched quote
- Global + user-level live-trading gates, all defaulting to off

### 3.3 Data and persistence

- PostgreSQL schema covering `users`, `mt5_accounts`, `ai_analyses`, `trade_intents`, `orders`, `positions_snapshot`, `risk_profiles`, `user_settings`, `audit_logs`, `idempotency_keys`, plus the billing tables `pricing_plans`, `subscriptions`, `invoices` (seeded with proposed plan data)
- Migrations are embedded into the compiled binary at build time (`sqlx::migrate!`) and run automatically on startup — no manual migration step

### 3.4 Security foundations

- AES-256-GCM credential encryption utility with an externally-stored key and stored key version (for future rotation)
- Internal MT5 Bridge authentication via `X-Internal-API-Key`
- Admin dashboard authentication via HTTP Basic + Argon2 password hash
- Audit-event persistence for material actions (analysis, intent lifecycle, execution, kill switch)
- A documented threat model ([security.md](security.md)) kept current as capabilities are added

### 3.5 MT5 Bridge (mock mode)

- Python/FastAPI bridge implementing `GET /health`, `GET /accounts/{id}/quote/{symbol}`, `POST /accounts/{id}/orders`
- Mock quote/order execution, used by default in every environment that doesn't have a real Windows MT5 terminal attached
- Per-account async lock in place as a foundation for the isolation a production deployment will need (see [§5](#5-not-started))

### 3.6 Admin dashboard

- Read-only operations console at `/admin/`, served directly from the Rust binary (HTML compiled in — no separate deploy step)
- System health, user/account counts, daily analysis/trade/order counts, subscription and invoice-revenue summary, recent audit events, searchable/filterable user-billing table

### 3.7 Testing

- Rust unit tests covering credential encryption, safe demo risk input, live-account rejection under safe defaults, and invalid BUY stop-loss rejection
- Python syntax validation for the MT5 Bridge (`python -m compileall`)

This is a real but limited safety net — see [§5](#5-not-started) for what's still missing here.

### 3.8 Licensing and project documentation

- `LICENSE` — PolyForm Noncommercial License 1.0.0: free for noncommercial use (learning, research, security review); commercial use requires a separate agreement with the owner
- A complete documentation set covering business requirements ([BRD](brd.md)), product requirements ([PRD](prd.md)), architecture, the Rust/Python split, security, risk controls, the API, pricing/billing design, SaaS/administration/AI/market-data model, the Telegram command specification, user-facing safety guidance, the relationship between MT5 and the broker, and deployment guides for both a Linux VPS (Docker or native) and the Windows host a real MT5 connection requires — see the full index in the [README](../README.md)

## 4. Partial / In Progress

| Area | What exists | What's missing |
|---|---|---|
| Billing | Database schema (`pricing_plans`, `subscriptions`, `invoices`) seeded with proposed plans; admin dashboard reads billing state | No `/plans` or checkout flow, no payment-gateway integration, no webhook verification, no usage metering, no refund/cancellation handling — see [pricing-billing-and-payment-administration.md §17](pricing-billing-and-payment-administration.md#17-implementation-phases) |
| Admin dashboard | Read-only views and filters work today | No role-based access control, no MFA, no mutation actions (refund, credit, cancellation, suspend) — still an MVP console per [admin-dashboard.md](admin-dashboard.md) |
| MT5 account model | Account type (`DEMO`/`LIVE`), permission mode, and encrypted-credential storage are all modeled and enforced | No public endpoint yet for a user to actually submit those details — see [§5](#5-not-started) |
| Risk engine | Ownership/permission/lot/slippage/live-gate checks are real | Daily-loss amount/percentage and per-trade-risk-percentage fields exist in the schema but are **not yet wired to authoritative broker equity or deal history** — their presence in the database is not enforcement (explicitly flagged in [risk-controls.md](risk-controls.md)) |
| Multi-account isolation | A per-account lock exists in the bridge | The bridge doesn't yet route by account to separate terminal processes — one bridge process currently serves whichever single account is logged into its one MT5 terminal (see [windows-vps-deployment.md §5](windows-vps-deployment.md#5-current-code-limitation-one-terminal--one-account)) |

## 5. Not Started

- **Telegram bot interface.** The entire user-facing product — `/start`, `/analyze`, `/buy`, `/sell`, `/positions`, `/history`, `/accounts`, `/risk`, `/stoptrading`, etc. — is specified in detail in [telegram-command-reference.md](telegram-command-reference.md) but has no Teloxide implementation yet. Nobody can currently use this product through Telegram.
- **MT5 account connection / onboarding API.** No endpoint exists for a user to submit broker, login, password, server, and account type and have it verified and stored.
- **Positions, history, and position management.** No endpoints for viewing open positions or trade history, and no confirmed close/modify-SL/modify-TP flow.
- **Full broker-derived risk calculations.** Margin, broker min/max/step volume, symbol trading status, and market-session validation against live MT5 data are not implemented; neither is the daily-loss enforcement noted in §4.
- **Real MT5 connectivity.** The current stack only runs in `MT5_MODE=MOCK`. Connecting an actual demo or live broker account requires a Windows host for the MT5 terminal and the official `MetaTrader5` Python package — this is a platform constraint, not unfinished code (see [linux-vps-deployment.md](linux-vps-deployment.md) and [windows-vps-deployment.md](windows-vps-deployment.md), which document exactly how to do this when it's time, but it hasn't been executed against a real broker in this repository).
- **Production multi-account MT5 worker orchestration.** One isolated terminal/worker process per active account, with Rust-side routing from `account_id` to the correct bridge instance — currently just a design requirement, not implemented.
- **Payment gateway integration.** No hosted checkout, no signed-webhook processing, no reconciliation job — only the target database schema exists.
- **Admin RBAC, MFA, and mutation actions.** The dashboard is read-only; role separation (owner/security/ops/support/auditor) and safe billing actions (refund, credit, cancellation) are documented as requirements but not built.
- **End-to-end integration test suite.** Current tests are unit-level only; there is no automated coverage exercising the full analysis → confirm → execute path against a real database, no cross-user isolation test suite, and no Telegram/payment/broker-failure test coverage.
- **Legal, regulatory, and compliance review.** Required before any commercial or live-money launch, per [BRD §7.8](brd.md#78-compliance-legal-and-regulatory) — not started, and not something engineering work alone can complete.

## 6. Why These Aren't Done Yet (Not Just a Backlog)

A few of the "not started" items aren't simply unscheduled work — they depend on things outside the codebase:

- **Real MT5 connectivity** needs an actual Windows machine (or a broker-provided Windows MT5 VPS) to be provisioned and operated — a deployment/infrastructure decision, not a coding task, and the guides for it already exist and are ready to follow once that decision is made.
- **Payment integration** needs a payment-gateway merchant account (e.g., Midtrans, Xendit, or Stripe) to be opened by the operator first — engineering can't proceed meaningfully without that business step happening first.
- **Legal/compliance review** needs a licensed professional, not more code.

Everything else in §5 is ordinary engineering backlog and can proceed independently.

## 7. Suggested Next Priorities

In rough dependency order (mirrors [PRD §11](prd.md#11-release-phases)):

1. **Telegram product surface** — this is the single highest-leverage gap: none of the working backend is usable by an actual person until this exists.
2. **MT5 account onboarding + positions/history/close** — completes the core user journey the Telegram layer needs to expose.
3. **Billing foundation and hosted checkout** — needed before any commercial launch, independent of MT5/Telegram work.
4. **Production MT5 worker isolation** — needed before more than one real account can be supported safely.
5. **Compliance, security review, and legal sign-off** — gate live-money trading regardless of how much of the above is done.

## 8. Related Documentation

- [Product requirements document](prd.md) — full functional/non-functional requirement status and phased roadmap
- [Business requirements document](brd.md) — the commercial and compliance requirements behind what's still missing
- [Product overview](product-overview.md) — the product's own account of its current status
- [Admin dashboard](admin-dashboard.md), [pricing/billing design](pricing-billing-and-payment-administration.md) — detail behind the "Partial" items in §4
- [README](../README.md) — the canonical, most frequently updated implementation-status summary and full documentation index
