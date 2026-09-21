# Windows MT5 bridge: code and deployment contract

The native bridge is designed as **one process and one portable MT5 terminal per account**. Install the official MT5 terminal and the Python `MetaTrader5` package on Windows. The Linux Docker bridge stays in `MOCK` mode for the existing Telegram demo.

## Provision an account

1. Submit the account through `POST /api/v1/mt5/accounts` using signed Telegram Mini App `initData`. Record the returned UUID.
2. Install a separate MT5 terminal copy for this account. Note the absolute `terminal64.exe` path. Do not reuse a terminal path for another bridge process.
3. Start a bridge process with these environment variables:

   ```text
   MT5_MODE=DEMO
   MT5_ACCOUNT_ID=<account UUID>
   MT5_TERMINAL_PATH=C:\MT5\account-1\terminal64.exe
   MT5_BRIDGE_API_KEY=<same private secret as Rust>
   ```

   Run `python -m uvicorn app.main:app --host <private VPN IP> --port <unique port> --workers 1` from `mt5-bridge`. Keep the terminal and bridge under the same dedicated Windows user. Use a service manager for restart, but after a restart call verification again: the bridge deliberately does not persist plaintext credentials or silently reuse a remembered account.
4. Set `MT5_BRIDGE_ROUTES` on the Rust backend to a JSON object mapping each account UUID to its bridge URL, for example `{"<account UUID>":"http://<private VPN IP>:8001"}`. Use `MT5_MODE=DEMO` on the Rust backend so an unmapped account fails closed. Restart Rust after editing routes.
5. Call `POST /api/v1/mt5/accounts/{id}/verify` with fresh signed Telegram Mini App `initData`. The bridge logs in with the stored encrypted credentials, and Rust checks the returned login and server before recording verification.

Each native bridge rejects any other `account_id`. Before every quote and order it checks that the terminal still reports the verified login and server. Bridge errors do not include passwords or raw MT5 responses. The private key header is required for all account operations; expose the bridge only over a private VPN or tunnel.

## Current trading boundary

Verification leaves the account `READ_ONLY`. The existing order risk engine has no broker-derived daily loss and margin checks, so this deployment path is for verifying connectivity and fetching account quotes; it does **not** turn on live or demo MT5 order execution. Keep `LIVE_TRADING_ENABLED=false`. There is no automated Windows VPS integration test until an actual terminal and broker demo account are available.
