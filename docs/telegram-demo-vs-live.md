# Can Telegram Users Tell If They're on a Demo or Live Account?

## 1. Direct Answer

**Yes — and by design, it is never left ambiguous.** The specification for this application's Telegram interface requires the account mode (`DEMO` or `LIVE`) to be shown redundantly, on multiple different screens, using the literal word `DEMO` or `LIVE` rather than a subtle icon or color alone. This is intentional: accidentally trading real money while believing you're on a demo account is exactly the class of mistake this product's entire confirmation and risk-gate architecture exists to prevent (see [product-overview.md §2.3](product-overview.md#23-final-review-and-explicit-confirmation) and [§8 Trust and Safety Model](product-overview.md#8-trust-and-safety-model)).

> **Implementation status:** the screens and message formats described below come from [telegram-command-reference.md](telegram-command-reference.md), the functional specification for the Telegram bot. The current repository implements the underlying Rust HTTP foundation (analyses, trade intents, confirmation, kill switch); the Teloxide command/callback layer that renders these exact screens in Telegram is not implemented yet. What follows is the required behavior once it is built, not a description of a currently running bot.

## 2. Why the Mode Must Always Be Visible

The product's core operating principle is that a user must always understand what they are about to confirm (see [product-overview.md](product-overview.md#5-core-value-proposition)). Mode confusion is one of the most damaging possible failures for a forex tool — see [how-to-use-the-application.md §10.2](how-to-use-the-application.md#102-main-risk-categories), which lists "operational risk" (incorrect account, symbol, lot, or mode) as a distinct risk category with the stated mitigation: *"Review every field and require explicit confirmation."*

Because of this, the specification does not rely on the user remembering which account they picked earlier in a conversation. It requires the mode to be redisplayed at every point where it matters.

## 3. Where the Mode Is Shown

### 3.1 The main menu — always-visible status line

Every time a user opens `/menu`, the bot shows a compact status summary that includes the mode, per [telegram-command-reference.md §5](telegram-command-reference.md#5-menu):

```text
Mode: DEMO
Account: *****678
Trading: ENABLED
Live trading: DISABLED
Subscription: PRO
```

This is the cheapest way for a user to check their current mode at any time — just send `/menu`.

### 3.2 `/accounts` — every connected account lists its explicit type

`/accounts` lists each connected MT5 account with its own labeled `Type` field, per [telegram-command-reference.md §11](telegram-command-reference.md#11-accounts):

```text
MT5 ACCOUNTS

1. Personal Demo
   Broker: Example Broker
   Login: *****678
   Server: Example-Demo
   Type: DEMO
   Permission: READ_ONLY
   Status: VERIFIED
```

If a user connects more than one account (e.g., one demo and one live), this screen is where they can see all of them side by side, each with its own explicit type.

### 3.3 The trade review screen — shown immediately before every confirmation

This is the most important place the mode appears, because it is the last thing the user sees before pressing a confirm button. Per [telegram-command-reference.md §7](telegram-command-reference.md#7-buy) (identical structure for `/sell` in §8):

```text
CONFIRM TRADE

Mode: DEMO
Account: *****678
Broker: Example Broker
Symbol: EURUSD
Side: BUY
Lot: 0.01
Current price: 1.1752
Stop loss: 1.1710
Take profit: 1.1810
Estimated risk: IDR or account-currency amount
Risk/reward: 1 : 1.5

No order has been placed yet.
```

For a **live** account, the specification requires two additional, deliberately harder-to-miss changes on this exact screen:

1. The confirmation button itself changes text — from **CONFIRM BUY** to **CONFIRM LIVE BUY** (or **CONFIRM SELL** to **CONFIRM LIVE SELL**), so the word "LIVE" appears on the button the user actually presses, not just in a paragraph above it.
2. The screen displays a **LIVE MONEY WARNING** banner (see [telegram-user-guide.md](telegram-user-guide.md) and [product-overview.md §2.3](product-overview.md#23-final-review-and-explicit-confirmation)).

So a demo confirmation and a live confirmation are not the same screen with one word changed quietly — the button wording and the warning banner are both different, on top of the `Mode:` field.

### 3.4 `/risk` — mode-relevant flags in the risk settings screen

`/risk` shows the user's `Trading` and `Live trading` flags together, per [telegram-command-reference.md §13](telegram-command-reference.md#13-risk):

```text
RISK SETTINGS

Account: *****678
Maximum lot: 0.10
Maximum open positions: 5
Maximum trades per day: 20
Maximum daily loss: 3%
Maximum risk per trade: 1%
Maximum slippage: 20 points
Trading: ENABLED
Live trading: DISABLED
```

`Live trading: DISABLED` here reflects the user-level gate described in [risk-controls.md](risk-controls.md) — one of several independent switches that must all be on before a live order can ever be submitted, regardless of which account is selected.

## 4. Where the Mode Actually Comes From

The `DEMO`/`LIVE` label shown on every screen above is not guessed or inferred — it is an explicit field the user declares when connecting the account, and the system deliberately refuses to derive it from anything else:

> "The account type is stored explicitly and must be verified. The application never infers whether an account is demo or live only from its server name." — [MT5 and the dual tech stack §2.3](mt5-and-dual-tech-stack.md#23-account-identity)

When adding an account through `/accounts`, the user must explicitly choose `DEMO` or `LIVE` as one of the required fields (alongside broker, login, server, and password) — see [telegram-command-reference.md §11 "Adding an account"](telegram-command-reference.md#adding-an-account). This value is then what every later screen displays back to the user — it is a stored, authoritative database field (`mt5_accounts.account_type` — see [user-journey-and-data-flow.md §6](user-journey-and-data-flow.md#6-entity-relationship-diagram)), not something recomputed by guesswork each time.

## 5. Cross-Checking Against MT5 Directly

Because the label ultimately depends on what the user declared when connecting the account, the user guide adds an important independent check: **compare what the bot shows against what MT5 itself shows**, not just trust the bot's label in isolation.

- During account setup: *"After verification, check that the displayed broker, masked account number, server, balance, equity, and account type match the information shown directly in MT5. Stop if anything differs."* — [how-to-use-the-application.md §4](how-to-use-the-application.md#4-intended-end-user-setup)
- Before every confirmation: the first item on the final-review checklist is *"I checked whether the account is `DEMO` or `LIVE`."* — [how-to-use-the-application.md §15](how-to-use-the-application.md#15-final-safety-checklist)

This cross-check exists because the bot's display is only as correct as the account-type field it was given; independently confirming against MT5 catches the case where that field was mis-declared at setup time.

## 6. Summary Table

| Where | What you see | Reference |
|---|---|---|
| `/menu` | `Mode: DEMO` (or `LIVE`) in the compact status line | [telegram-command-reference.md §5](telegram-command-reference.md#5-menu) |
| `/accounts` | `Type: DEMO` (or `LIVE`) listed per connected account | [telegram-command-reference.md §11](telegram-command-reference.md#11-accounts) |
| `/buy` or `/sell` review | `Mode: DEMO` (or `LIVE`) field, plus for live: a `CONFIRM LIVE BUY`/`CONFIRM LIVE SELL` button and a `LIVE MONEY WARNING` banner | [telegram-command-reference.md §7–8](telegram-command-reference.md#7-buy) |
| `/risk` | `Live trading: ENABLED`/`DISABLED` gate status | [telegram-command-reference.md §13](telegram-command-reference.md#13-risk) |
| MetaTrader 5 itself | Independent, authoritative cross-check of broker, account, server, and type | [how-to-use-the-application.md §4](how-to-use-the-application.md#4-intended-end-user-setup) |

## 7. What Actually Changes Between Demo and Live (Beyond the Label)

The visible label is only the surface of a much larger distinction. A live order additionally requires every one of these independent gates to pass — any single one being off blocks execution, regardless of what the screen shows:

1. Deployment runs with `TRADING_MODE=LIVE`.
2. The global `LIVE_TRADING_ENABLED` flag is `true`.
3. The user's own `live_trading_enabled` setting is `true`.
4. The specific account is explicitly recorded and verified as `LIVE`.
5. The account has `TRADING_ENABLED` permission.
6. The user's risk-profile kill switch allows new trades.
7. The user presses the explicit, distinctly-labeled live confirmation button.

Full detail: [user-journey-and-data-flow.md §9 "Demo-to-Live Journey"](user-journey-and-data-flow.md#9-demo-to-live-journey) and [risk-controls.md](risk-controls.md). The on-screen `DEMO`/`LIVE` label is what tells the *user* which mode they're in; these seven gates are what actually control whether real money can move, independent of the label.

## 8. Frequently Asked Questions

### If I have both a demo and a live account connected, can I tell which one a trade review is about?

Yes — the `Account:` line (masked login) and `Mode:` line on the trade review screen (§3.3) identify exactly which connected account the pending trade belongs to, and for a live account the button and warning banner make it unmistakable.

### Could the bot ever show "DEMO" for an account that's actually live, or vice versa?

Only if the account type was mis-declared when it was connected, since the label is read from that stored field rather than inferred. This is exactly why §5's cross-check against MT5 itself exists — treat it as a required step, not an optional one, especially the first time you connect an account.

### Is there a way to see the mode without starting a trade?

Yes — `/menu` and `/accounts` both show it at any time, with no trade in progress.

### Does a "DEMO" label mean nothing bad can happen?

It means no real broker funds are at risk for that specific account. It does not mean the workflow, risk checks, or confirmation requirements are skipped — the same review-and-confirm process applies in demo mode so that it accurately rehearses the live experience.

## 9. Related Documentation

- [Telegram Bot Command Reference](telegram-command-reference.md) — full message specifications quoted above
- [Telegram user guide](telegram-user-guide.md) — condensed onboarding and confirmation rules
- [Application user guide and safety FAQ](how-to-use-the-application.md) — the independent MT5 cross-check and safety checklist
- [MT5 and the Rust–Python architecture](mt5-and-dual-tech-stack.md) — why account type is explicit, never inferred
- [Risk controls](risk-controls.md) — the live-trading gates behind the `Live trading` status flag
- [User journey and data flow](user-journey-and-data-flow.md) — the demo-to-live progression
