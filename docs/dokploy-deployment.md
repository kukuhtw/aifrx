# Deploying with Dokploy

This deployment runs the current HTTP API, admin dashboard, PostgreSQL, and the
MT5 bridge in mock mode. A Linux Dokploy host cannot run the native MetaTrader 5
Python integration. Real demo/live MT5 needs the separate Windows bridge
described in [windows-vps-deployment.md](windows-vps-deployment.md).

## 1. Create the Compose service

1. In Dokploy, create a project and add a **Compose** service.
2. Select **Docker Compose**, not Docker Stack. Stack mode cannot build the two
   repository Dockerfiles.
3. Connect this Git repository and select the deployment branch.
4. Set **Compose Path** to `./docker-compose.dokploy.yml`.
5. Enable **Isolated Deployments** when available. The application also has its
   own internal network for database and bridge traffic.

Do not configure a public domain for `postgres` or `mt5-bridge`.

## 2. Configure the environment

Paste the following into the Compose service's **Environment** tab, replacing
every placeholder. Dokploy writes these values to the Compose deployment's
`.env` file.

```dotenv
POSTGRES_PASSWORD=<strong-password-without-url-special-characters>
DATABASE_URL=postgres://aiforex:<same-password>@postgres:5432/aiforex
MT5_BRIDGE_API_KEY=<64-character-random-hex-value>
ENCRYPTION_KEY=<base64-encoded-32-byte-value>
OPENAI_API_KEY=
OPENAI_MODEL=gpt-5-mini
TELEGRAM_BOT_TOKEN=<token-from-botfather>
TRADING_MODE=DEMO
MARKET_DATA_MAX_AGE_SECONDS=30
RUST_LOG=info
ADMIN_USERNAME=admin
ADMIN_PASSWORD_HASH_B64=<base64-encoded-argon2-hash>
```

Generate secrets on a trusted machine:

```bash
openssl rand -base64 32
openssl rand -hex 32
cargo run -p ai-forex-backend --bin hash-admin-password
```

`POSTGRES_PASSWORD` and the password embedded in `DATABASE_URL` must match. If a
password contains URL-reserved characters, percent-encode it in `DATABASE_URL`.
Keep `TRADING_MODE=DEMO`; the Compose file deliberately fixes
`LIVE_TRADING_ENABLED=false` and `MT5_MODE=MOCK`.

When `TELEGRAM_BOT_TOKEN` is present, the Rust backend starts the bot using
Telegram long polling. Only one running backend instance may poll a bot token.
Create the bot with `@BotFather`, paste its token here, redeploy, then open the
bot's private chat and send `/start`. Leave the value empty to disable Telegram.

## 3. Deploy and attach the domain

1. Deploy once and confirm all three services become healthy.
2. Open **Domains**, add the API hostname, and select service `rust-backend` with
   container port `8080` and path `/`.
3. Enable HTTPS with Let's Encrypt after the hostname's DNS A/AAAA record points
   to the Dokploy server.
4. Redeploy the Compose service after adding or changing a domain; Dokploy adds
   Compose routing labels during deployment.

No host port needs to be published. Traefik reaches port 8080 through Dokploy's
deployment network.

## 4. Verify

```bash
curl --fail https://api.example.com/health
```

Expected response:

```json
{"status":"ok","database":"ok","mt5_bridge":"ok"}
```

Then check `https://api.example.com/admin/`. It should request HTTP Basic
authentication. If `ADMIN_PASSWORD_HASH_B64` is empty or invalid, the admin
dashboard intentionally remains unavailable.

## 5. Operations

- Enable auto-deploy only for the intended production branch.
- Configure a scheduled Dokploy backup for the `postgres-data` named volume.
  Dokploy will display its actual name as `<compose-app-name>_postgres-data`.
- Keep at least one tested off-server backup before upgrades.
- Review `rust-backend`, `mt5-bridge`, and `postgres` logs after every deploy.
- Treat a non-200 `/health` response as a deployment failure.
- Never expose port 5432, port 8000, `.env`, or either internal secret publicly.

The current repository is a mock/demo foundation, not a complete public trading
product. Telegram, payment processing, production multi-account MT5 isolation,
MFA, and RBAC remain outside this deployment.
