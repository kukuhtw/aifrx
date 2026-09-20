# Security and threat model

Trust boundaries are Telegram, OpenAI, PostgreSQL, the private bridge, MT5, and the broker. AI and broker responses are untrusted. Only approved quote fields enter the AI request; credentials, tokens and account identity do not. AI output is schema constrained and validated again in Rust.

MT5 passwords must be encrypted using AES-256-GCM with a fresh nonce. The base64 32-byte key stays outside PostgreSQL and supports a stored key version for rotation. Never log credential request bodies. Internal bridge calls require `X-Internal-API-Key`; deploy the bridge without a public port, preferably with mTLS and one Windows terminal process per active account.

Primary threats and mitigations:

| Threat | Control |
|---|---|
| Cross-user access | Every account query binds both account and authenticated user IDs; isolation tests required |
| Duplicate trade | Transaction, row lock, unique idempotency hash and unique order per intent |
| Prompt injection | Typed allowlisted data only; no user text in system prompt; output never invokes tools |
| Stale/manipulated price | Timestamp freshness and immediate pre-order quote/slippage validation |
| Live-order accident | Global, runtime-mode, user, account permission and verification gates |
| Credential disclosure | AEAD encryption, external key, redacted logs and private bridge |
| Shared MT5 session confusion | Per-account lock; production requires process/terminal isolation |
| Admin account compromise | Role-based access control (5 least-privilege roles), mandatory TOTP MFA, short-lived cookie sessions with CSRF protection, immutable per-action audit trail — see [admin-dashboard.md](admin-dashboard.md) |

This system is not unhackable. Use secret rotation, least-privilege networking, dependency scanning, encrypted backups, alerting, penetration testing, and legal/compliance review before production.

