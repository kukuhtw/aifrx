# Admin Dashboard

## Overview

The application includes a read-only operations dashboard at `/admin/`. It gives authorized operators visibility into platform health, users, MT5 accounts, trading activity, subscriptions, invoices, and recent audit events.

The dashboard deliberately does not expose MT5 passwords, API keys, broker credential payloads, or trade-execution controls.

## Features

- Database and MT5 Bridge health indicators
- Global live-trading feature-flag status
- Total and active user counts
- Total and verified MT5 account counts
- Daily analysis and trade-intent counts
- Daily executed and failed order counts
- Active, trial, past-due, and pending subscription counts
- Paid invoice revenue for the current month
- Recent audit events
- Searchable user and billing table
- Subscription-status filtering
- Responsive desktop and mobile layout

## Configure Administrator Access

The dashboard uses HTTP Basic authentication with an Argon2 password hash. It must be served only over HTTPS in production.

Generate a password hash interactively:

```powershell
cargo run -p ai-forex-backend --bin hash-admin-password
```

The command requires at least 12 characters and prints a base64-encoded Argon2 hash. Copy the result into `.env`:

```dotenv
ADMIN_USERNAME=admin
ADMIN_PASSWORD_HASH_B64=<generated-value>
```

The raw password is not written to `.env` or PostgreSQL. If `ADMIN_PASSWORD_HASH_B64` is missing, the dashboard returns `503 Service Unavailable` and remains inaccessible.

Restart the Rust backend after changing these variables.

## Open the Dashboard

For local development, navigate to:

```text
http://localhost:8080/admin/
```

The browser will request the configured administrator username and password.

Production requirements:

- Use HTTPS only.
- Restrict access through a VPN, private network, or identity-aware proxy.
- Add multi-factor authentication before commercial production.
- Use separate named administrator identities instead of one shared account.
- Rotate credentials and audit access.
- Apply stricter rate limiting to `/admin/*`.

HTTP Basic authentication is appropriate only for the current protected MVP console. A production admin identity system should use short-lived sessions, MFA, role-based access control, CSRF protection, session revocation, and per-administrator audit records.

## Billing Data

Migration `0002_billing.sql` creates:

- `pricing_plans`
- `subscriptions`
- `invoices`

It also seeds the proposed Free Trial, Demo, Pro, and Business plans. These plan prices remain proposals until formally approved.

Users without a subscription appear as `UNPAID`. The dashboard determines billing state from PostgreSQL records populated by verified payment-provider webhooks. Payment screenshots and browser redirects must never be used as authoritative payment evidence.

## Dashboard API

Both endpoints require administrator authentication:

- `GET /admin/api/overview`
- `GET /admin/api/users?search=&subscription=&limit=50&offset=0`

Supported subscription filters include:

- `ACTIVE`
- `TRIALING`
- `PENDING_PAYMENT`
- `PAST_DUE`
- `SUSPENDED`
- `CANCELLED`
- `EXPIRED`
- `UNPAID`

The API never returns encrypted passwords, nonces, full broker responses, or service secrets.

## Current Boundary

The dashboard is read-only. Payment-provider integration, webhook ingestion, refunds, subscription mutations, administrator role management, and MFA are not implemented yet.

The dashboard must never become an alternative path for placing orders or silently enabling live trading for users.

