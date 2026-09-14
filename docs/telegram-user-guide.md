# Telegram user guide

Onboarding must show: “Your funds remain with your broker. This application does not receive or store your trading funds. It provides AI-assisted analysis and sends only trading instructions you explicitly confirm.” Also show the substantial-risk disclaimer before live enablement.

Recommended commands: `/start`, `/menu`, `/analyze`, `/buy`, `/sell`, `/positions`, `/history`, `/accounts`, `/security`, `/risk`, `/stoptrading`, `/resumetrading`, `/settings`, `/help`.

For syntax, prerequisites, expected responses, confirmation behavior, and security rules for each command, see the complete [Telegram Bot Command Reference](telegram-command-reference.md).

BUY/SELL creates an intent, never an immediate order. The bot then gathers lot, SL and TP and displays a final review. Live review must say `LIVE MONEY WARNING`. Only a distinct confirm button may execute. Always show an MT5 ticket on success and tell the user it can be verified in MetaTrader 5.
