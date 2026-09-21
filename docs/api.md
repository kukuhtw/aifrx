# HTTP API

All public deployment traffic should terminate behind authenticated Telegram webhook/API middleware and rate limiting. UUID `user_id` values shown here are trusted server-side identities, not values that a public client should be allowed to choose.

`POST /api/v1/analyses`: `{"user_id":"uuid","account_id":"uuid","symbol":"EURUSD","timeframe":"H1"}`.

`POST /api/v1/trade-intents`: `{"user_id":"uuid","account_id":"uuid","analysis_id":null,"symbol":"EURUSD","side":"BUY","volume":"0.01","stop_loss":"1.0700","take_profit":"1.0850"}`. This never sends an order.

`POST /api/v1/trade-intents/{id}/confirm`: `{"user_id":"uuid","idempotency_key":"client-generated-unique-value"}`. A duplicate key is rejected. Confirmation expires after five minutes.

`POST /api/v1/trading/stop`: `{"user_id":"uuid"}`. Blocks new orders but deliberately does not block position closing.

Errors never contain stack traces, secrets, bridge URLs, or broker credential payloads.
# MT5 account submission

`POST /api/v1/mt5/accounts` accepts `X-Telegram-Init-Data` containing the Telegram Mini App's raw `initData` string (valid for one hour). The Telegram user must have sent `/start` first. Body:

```json
{"broker":"Example Broker","login":"12345678","password":"MT5 password","server":"ExampleBroker-Demo","account_type":"DEMO"}
```

`account_type` must be `DEMO` or `LIVE`. The server derives ownership from the signed Telegram data, encrypts the password, and returns the account ID, broker, server, last four login digits, `is_verified: false`, and `permission_mode: READ_ONLY`. The password is never returned. A duplicate server/login for the same user is rejected.

`POST /api/v1/mt5/accounts/{id}/verify` uses the same `X-Telegram-Init-Data` header and no body. The account must belong to that Telegram user. Rust decrypts its password and sends it only to the assigned private Windows bridge. The bridge logs in to the bound MT5 terminal and returns broker-derived identity. If login and server match, Rust marks the account verified, updates the broker and account type from MT5, and leaves permissions `READ_ONLY`. A Windows bridge route must be configured before calling this endpoint. Verification does not enable order execution.
