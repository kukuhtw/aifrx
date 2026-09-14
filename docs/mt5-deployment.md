# MT5 deployment

For a conceptual explanation of MetaTrader 5, the bridge, and the dual technology stack, see [MetaTrader 5, the MT5 Bridge, and the Rust–Python Architecture](mt5-and-dual-tech-stack.md).

The Python container runs in mock mode on Linux. Real MetaTrader 5 uses the Windows-only terminal/package path and must run on a private Windows worker. Provision one OS user/terminal data directory and worker process per active account, or an equivalently isolated pool with strict identity checks. Never asynchronously call `mt5.login()` to swap users in one global process.

Expose the worker only to the Rust backend, require the internal key (prefer mTLS), restrict outbound traffic to intended broker endpoints, and never return or log passwords. Health should distinguish process availability from account authentication. Validate broker symbol metadata, filling policy, margin, and result codes before order submission.
