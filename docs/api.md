# HTTP API

All public deployment traffic should terminate behind authenticated Telegram webhook/API middleware and rate limiting. UUID `user_id` values shown here are trusted server-side identities, not values that a public client should be allowed to choose.

`POST /api/v1/analyses`: `{"user_id":"uuid","account_id":"uuid","symbol":"EURUSD","timeframe":"H1"}`.

`POST /api/v1/trade-intents`: `{"user_id":"uuid","account_id":"uuid","analysis_id":null,"symbol":"EURUSD","side":"BUY","volume":"0.01","stop_loss":"1.0700","take_profit":"1.0850"}`. This never sends an order.

`POST /api/v1/trade-intents/{id}/confirm`: `{"user_id":"uuid","idempotency_key":"client-generated-unique-value"}`. A duplicate key is rejected. Confirmation expires after five minutes.

`POST /api/v1/trading/stop`: `{"user_id":"uuid"}`. Blocks new orders but deliberately does not block position closing.

Errors never contain stack traces, secrets, bridge URLs, or broker credential payloads.

