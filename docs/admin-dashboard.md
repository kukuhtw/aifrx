# Admin Dashboard

## Overview

The application includes an operations dashboard at `/admin/`. It gives authorized operators visibility into platform health, Telegram bot setup, users, MT5 accounts, trading activity, subscriptions, invoices, and audit events, plus a small set of auditable mutation actions (suspend/reactivate a user, schedule a subscription cancellation, grant a promotional credit, mark an invoice as refunded).

The **Telegram bot setup** panel shows whether `TELEGRAM_BOT_TOKEN` is configured, instructions for creating a bot with @BotFather and starting the demo flow, and a direct `t.me` link when `TELEGRAM_BOT_USERNAME` is also configured. It never exposes the bot token. “Configured” indicates token presence, not a successful Telegram API connection.

The dashboard deliberately does not expose MT5 passwords, API keys, broker credential payloads, or trade-execution controls. No administrative action can place, close, or modify a trade, and no action silently enables live trading for a user.

## Authentication and MFA

Admin identities are stored in PostgreSQL (`admin_users`), not in environment variables. Each admin account has:

- A username and an Argon2 password hash.
- A role (see [Roles](#roles-and-permissions) below).
- A per-admin TOTP (RFC 6238) secret, encrypted at rest with the same AES-256-GCM key that protects MT5 credentials. TOTP is mandatory — there is no way to create an admin account without one.

Signing in at `/admin/` requires username, password, **and** a current 6-digit authenticator code in a single request (`POST /admin/auth/login`). On success the server issues a short-lived (8-hour) session, delivered as an `HttpOnly`, `Secure`, `SameSite=Strict` cookie scoped to `/admin`. Every mutating request must also carry the session's CSRF token in an `X-Admin-Csrf-Token` header (returned at login and by `GET /admin/auth/session`); the dashboard's own JavaScript handles this automatically.

`POST /admin/auth/logout` revokes the session immediately.

The application must still be served only over HTTPS in production — the `Secure` cookie flag depends on it.

### Bootstrapping the first admin (OWNER)

Admin accounts are created with the `create-admin` CLI tool, not through the API (except for an `OWNER` creating further admins from the dashboard itself — see [Managing admins](#managing-admins)).

```powershell
cargo run -p ai-forex-backend --bin create-admin
```

or, against a running Docker/Dokploy deployment:

```bash
docker compose exec rust-backend create-admin
```

It prompts for a username, role, and temporary password, then prints a TOTP provisioning secret and `otpauth://` URL **once** — add it to an authenticator app (Google Authenticator, 1Password, Authy, etc.) immediately; it cannot be retrieved again. The tool applies pending migrations before inserting the row, so it's safe to run before the server has ever started.

## Roles and permissions

Source of truth: [saas-administration-ai-and-market-data.md §3.1](saas-administration-ai-and-market-data.md#31-recommended-administrative-roles).

| Role | Read (overview, users, billing state) | Suspend/reactivate user | Cancel subscription / grant credit / refund | Admin audit log | Manage admins |
|---|---|---|---|---|---|
| `OWNER` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `SECURITY_OPERATOR` | ✓ | – | – | ✓ | – |
| `OPS_ADMIN` | ✓ | ✓ | ✓ | – | – |
| `SUPPORT_AGENT` | ✓ | ✓ | – | – | – |
| `AUDITOR` | ✓ | – | – | ✓ | – |

Every role check happens server-side on each request; the dashboard only hides buttons a role can't use as a convenience.

## Mutation actions

All mutation endpoints require a `reason` (5–500 characters) and write an immutable row to `admin_audit_logs` (admin identity, action, target, reason, timestamp) — visible in the dashboard's "Admin audit" panel to `OWNER`/`SECURITY_OPERATOR`/`AUDITOR`.

- `POST /admin/api/users/{id}/suspend`, `POST /admin/api/users/{id}/reactivate`
- `POST /admin/api/subscriptions/{id}/cancel-at-period-end` — sets `CANCEL_AT_PERIOD_END`; access is preserved through the paid period, matching [BR-22](brd.md#7-detailed-requirements)
- `POST /admin/api/users/{id}/credit` `{amount_minor, reason}` — appends to a local `account_credits` ledger
- `POST /admin/api/invoices/{id}/refund` then `POST /admin/api/invoices/{id}/confirm-refund` — moves an invoice `PAID → REFUND_PENDING → REFUNDED`

**Important limitation:** there is no payment-gateway integration yet (Phase 3 of [pricing-billing-and-payment-administration.md §13](pricing-billing-and-payment-administration.md#13-implementation-phases) hasn't started). "Refund" and "credit" here are local, operator-attested database state changes with a mandatory audit reason — not automated, provider-verified transactions. Treat them as bookkeeping that a real payment-gateway refund/credit must still be issued to match, until that integration exists.

## Managing admins

`OWNER`-only, from the dashboard's "Admins" panel or directly:

- `GET /admin/api/admins`, `POST /admin/api/admins` `{username, temporary_password, role}` — generates and encrypts a new TOTP secret server-side and returns the provisioning secret/URL **once** in the response; hand it to the new admin out of band.
- `POST /admin/api/admins/{id}/role` `{role}`
- `POST /admin/api/admins/{id}/deactivate`, `POST /admin/api/admins/{id}/reactivate` — deactivating also revokes that admin's active sessions immediately.

An `OWNER` cannot deactivate or change the role of their own account through the API (prevents accidental lockout); use another `OWNER` account or the `create-admin` CLI for recovery.

## Open the Dashboard

For local development, navigate to:

```text
http://localhost:8080/admin/
```

The page itself is public (it renders a login form), but every `/admin/api/*` endpoint requires a valid session.

Production requirements:

- Use HTTPS only.
- Restrict network access through a VPN, private network, or identity-aware proxy in addition to the login/MFA gate.
- Use separate named administrator identities per person — never share one account.
- Rotate credentials and TOTP enrollments when an admin leaves.
- Apply stricter rate limiting to `/admin/*` (not yet implemented — see [Current Boundary](#current-boundary)).

## Billing Data

Migration `0002_billing.sql` creates:

- `pricing_plans`
- `subscriptions`
- `invoices`

It also seeds the proposed Free Trial, Demo, Pro, and Business plans. These plan prices remain proposals until formally approved.

Migration `0003_admin_rbac.sql` adds `admin_users`, `admin_sessions`, `admin_audit_logs`, and `account_credits`.

Users without a subscription appear as `UNPAID`. The dashboard determines billing state from PostgreSQL records populated by verified payment-provider webhooks. Payment screenshots and browser redirects must never be used as authoritative payment evidence.

## Dashboard API

All endpoints under `/admin/api/*` require a valid admin session (and, for mutations, a matching CSRF header):

- `GET /admin/api/overview`
- `GET /admin/api/users?search=&subscription=&limit=50&offset=0`
- `GET /admin/api/audit` — admin action history
- The mutation and admin-management endpoints listed above

Supported subscription filters include:

- `ACTIVE`
- `TRIALING`
- `PENDING_PAYMENT`
- `PAST_DUE`
- `SUSPENDED`
- `CANCELLED`
- `EXPIRED`
- `UNPAID`

The API never returns encrypted passwords, nonces, full broker responses, TOTP secrets (after creation), or other service secrets.

## Current Boundary

Implemented: RBAC (5 roles), mandatory TOTP MFA, short-lived cookie sessions with CSRF protection, and the mutation actions above, all with an immutable audit trail — closing [BR-26](brd.md#7-detailed-requirements)/[BR-27](brd.md#7-detailed-requirements) and [FR-9.4](prd.md#7-functional-requirements)/[FR-9.5](prd.md#7-functional-requirements)/[FR-9.6](prd.md#7-functional-requirements).

Still not implemented:

- Rate limiting / lockout on `/admin/auth/login`.
- Real payment-gateway-backed refund/credit automation (depends on Phase 3 billing integration).
- Self-service TOTP re-enrollment (a lost authenticator currently requires another `OWNER`, or the `create-admin` CLI for a fresh account, to recover).

The dashboard must never become an alternative path for placing orders or silently enabling live trading for users.
