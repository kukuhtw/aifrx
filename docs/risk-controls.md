# Risk controls

Every order is denied unless the account belongs to the user, is active and verified, has `TRADING_ENABLED`, and the user's kill switch is off. Checks include maximum lot, open-position count, trades per day, valid side-relative SL/TP, price freshness, live gates and maximum slippage.

The schema also reserves daily loss amount/percentage and per-trade percentage. These must be wired to authoritative broker equity and deal history before production; their mere presence is not enforcement. Margin, broker min/max/step, symbol trading status, and market session must likewise be validated against MT5 immediately before enabling non-mock trading.

`/stoptrading` semantics block BUY/SELL while retaining analysis, viewing and closing. Resume must be a separate explicit confirmation flow.

