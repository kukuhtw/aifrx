use std::sync::Arc;

use anyhow::Context;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::Row;
use teloxide::{
    dispatching::UpdateFilterExt,
    dptree,
    prelude::*,
    types::{BotCommand, CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message},
};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{Analysis, ConfirmIntent, Side},
    state::AppState,
};

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    let token = state
        .config
        .telegram_bot_token
        .clone()
        .context("Telegram token is missing")?;
    let bot = Bot::new(token);
    bot.set_my_commands(commands()).await?;

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(handle_message))
        .branch(Update::filter_callback_query().endpoint(handle_callback));

    tracing::info!("Telegram bot polling started");
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .build()
        .dispatch()
        .await;
    Ok(())
}

fn commands() -> Vec<BotCommand> {
    [
        ("start", "Daftar dan buat akun mock demo"),
        ("menu", "Tampilkan menu utama"),
        ("analyze", "Analisis: /analyze EURUSD H1"),
        ("buy", "Intent BUY: /buy EURUSD 0.01 SL TP"),
        ("sell", "Intent SELL: /sell EURUSD 0.01 SL TP"),
        ("status", "Tampilkan status akun dan trading"),
        ("history", "Tampilkan 10 order terakhir"),
        ("stoptrading", "Blokir order baru segera"),
        ("resumetrading", "Minta konfirmasi aktifkan trading demo"),
        ("security", "Tampilkan informasi keamanan"),
        ("help", "Tampilkan bantuan"),
    ]
    .into_iter()
    .map(|(command, description)| BotCommand::new(command, description))
    .collect()
}

async fn handle_message(bot: Bot, msg: Message, state: Arc<AppState>) -> HandlerResult {
    let Some(text) = msg.text() else {
        return Ok(());
    };
    if !text.starts_with('/') {
        return Ok(());
    }
    if !msg.chat.is_private() {
        bot.send_message(
            msg.chat.id,
            "Gunakan bot melalui private chat untuk melindungi data akun.",
        )
        .await?;
        return Ok(());
    }
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    let telegram_id = i64::try_from(from.id.0).context("Telegram user ID is too large")?;
    let mut parts = text.split_whitespace();
    let command = parts
        .next()
        .unwrap_or_default()
        .split('@')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let args: Vec<&str> = parts.collect();

    let result: anyhow::Result<String> = match command.as_str() {
        "/start" => start_user(
            &state,
            telegram_id,
            from.username.as_deref(),
            &from.first_name,
            from.language_code.as_deref(),
        )
        .await
        .map(|_| welcome().to_owned()),
        "/menu" | "/help" => Ok(help().to_owned()),
        "/status" => status(&state, telegram_id).await.map(str::to_owned),
        "/analyze" => analyze(&state, telegram_id, &args).await,
        "/buy" => create_intent(&state, telegram_id, Side::Buy, &args).await,
        "/sell" => create_intent(&state, telegram_id, Side::Sell, &args).await,
        "/history" => history(&state, telegram_id).await,
        "/stoptrading" => stop_trading(&state, telegram_id).await.map(str::to_owned),
        "/resumetrading" => {
            let user_id = user_id(&state, telegram_id).await?;
            bot.send_message(
                msg.chat.id,
                "Aktifkan kembali pembuatan order DEMO? Semua order tetap memerlukan konfirmasi terpisah.",
            )
            .reply_markup(InlineKeyboardMarkup::new([[InlineKeyboardButton::callback(
                "CONFIRM RESUME",
                format!("resume:{user_id}:{}", Utc::now().timestamp()),
            )]]))
            .await?;
            return Ok(());
        }
        "/security" => Ok(security().to_owned()),
        "/positions" | "/accounts" | "/risk" | "/settings" => Ok(
            "Fitur ini belum tersedia pada tahap Telegram MVP. Gunakan /status, /analyze, /history, dan alur intent demo."
                .to_owned(),
        ),
        _ => Ok("Perintah tidak dikenal. Gunakan /help.".to_owned()),
    };

    match result {
        Ok(reply) => {
            if matches!(command.as_str(), "/buy" | "/sell") && reply.starts_with("INTENT:") {
                let (id, review) = reply
                    .strip_prefix("INTENT:")
                    .and_then(|v| v.split_once('\n'))
                    .context("invalid intent response")?;
                bot.send_message(msg.chat.id, review)
                    .reply_markup(InlineKeyboardMarkup::new([[
                        InlineKeyboardButton::callback("CONFIRM ORDER", format!("confirm:{id}")),
                        InlineKeyboardButton::callback("CANCEL", format!("cancel:{id}")),
                    ]]))
                    .await?;
            } else {
                bot.send_message(msg.chat.id, reply).await?;
            }
        }
        Err(error) => {
            tracing::warn!(telegram_id, %error, "Telegram command rejected");
            bot.send_message(msg.chat.id, friendly_error(&error))
                .await?;
        }
    }
    Ok(())
}

async fn handle_callback(bot: Bot, query: CallbackQuery, state: Arc<AppState>) -> HandlerResult {
    bot.answer_callback_query(query.id.clone()).await?;
    let telegram_id = i64::try_from(query.from.id.0).context("Telegram user ID is too large")?;
    let Some(data) = query.data.as_deref() else {
        return Ok(());
    };
    let result = if let Some(raw) = data.strip_prefix("confirm:") {
        confirm_intent(&state, telegram_id, Uuid::parse_str(raw)?).await
    } else if let Some(raw) = data.strip_prefix("cancel:") {
        cancel_intent(&state, telegram_id, Uuid::parse_str(raw)?).await
    } else if let Some(raw) = data.strip_prefix("resume:") {
        let (user, issued_at) = raw.split_once(':').context("invalid resume callback")?;
        resume_trading(
            &state,
            telegram_id,
            Uuid::parse_str(user)?,
            issued_at.parse()?,
        )
        .await
    } else {
        Ok("Tindakan tidak dikenal.".to_owned())
    };
    let reply = result.unwrap_or_else(|error| {
        tracing::warn!(telegram_id, %error, "Telegram callback rejected");
        friendly_error(&error)
    });
    bot.send_message(query.from.id, reply).await?;
    Ok(())
}

async fn start_user(
    state: &AppState,
    telegram_id: i64,
    username: Option<&str>,
    first_name: &str,
    language: Option<&str>,
) -> anyhow::Result<()> {
    let mut tx = state.db.begin().await?;
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users(telegram_user_id,telegram_username,first_name,language) VALUES($1,$2,$3,$4) ON CONFLICT(telegram_user_id) DO UPDATE SET telegram_username=EXCLUDED.telegram_username,first_name=EXCLUDED.first_name,updated_at=now() RETURNING id",
    )
    .bind(telegram_id)
    .bind(username)
    .bind(first_name)
    .bind(language.unwrap_or("id"))
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO user_settings(user_id,language) VALUES($1,$2) ON CONFLICT(user_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(language.unwrap_or("id"))
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO risk_profiles(user_id,trading_enabled) VALUES($1,false) ON CONFLICT DO NOTHING")
        .bind(user_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO mt5_accounts(user_id,broker_name,server,login,encrypted_password,password_nonce,account_type,permission_mode,is_verified) VALUES($1,'Mock Broker','MOCK',$2,'MOCK','MOCK','DEMO','TRADING_ENABLED',true) ON CONFLICT(user_id,server,login) DO NOTHING")
        .bind(user_id).bind(format!("telegram-{telegram_id}")).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO audit_logs(user_id,event_type,entity_type,entity_id,metadata) VALUES($1,'TELEGRAM_USER_STARTED','user',$1,'{}')")
        .bind(user_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn user_id(state: &AppState, telegram_id: i64) -> anyhow::Result<Uuid> {
    sqlx::query_scalar("SELECT id FROM users WHERE telegram_user_id=$1 AND status='ACTIVE'")
        .bind(telegram_id)
        .fetch_optional(&state.db)
        .await?
        .context("Jalankan /start terlebih dahulu")
}

async fn account(state: &AppState, user_id: Uuid) -> anyhow::Result<Uuid> {
    sqlx::query_scalar("SELECT id FROM mt5_accounts WHERE user_id=$1 AND is_active AND is_verified ORDER BY created_at LIMIT 1")
        .bind(user_id).fetch_optional(&state.db).await?.context("Akun demo tidak tersedia; jalankan /start")
}

async fn status(state: &AppState, telegram_id: i64) -> anyhow::Result<&'static str> {
    let id = user_id(state, telegram_id).await?;
    let enabled: bool = sqlx::query_scalar("SELECT COALESCE((SELECT trading_enabled FROM risk_profiles WHERE user_id=$1 AND account_id IS NULL),false)")
        .bind(id).fetch_one(&state.db).await?;
    Ok(if enabled {
        "STATUS\nMode: DEMO / MT5 MOCK\nTrading intent: ENABLED\nLive trading: DISABLED"
    } else {
        "STATUS\nMode: DEMO / MT5 MOCK\nTrading intent: STOPPED\nLive trading: DISABLED\nGunakan /resumetrading untuk mengaktifkan demo."
    })
}

async fn analyze(state: &AppState, telegram_id: i64, args: &[&str]) -> anyhow::Result<String> {
    let symbol = args
        .first()
        .copied()
        .unwrap_or("EURUSD")
        .to_ascii_uppercase();
    let timeframe = args.get(1).copied().unwrap_or("H1").to_ascii_uppercase();
    validate_symbol_timeframe(&symbol, &timeframe)?;
    let user = user_id(state, telegram_id).await?;
    let account = account(state, user).await?;
    let quote = state
        .mt5
        .quote(account, &symbol)
        .await
        .map_err(anyhow::Error::new)?;
    let analysis = state
        .ai
        .analyze(&quote, &timeframe)
        .await
        .map_err(anyhow::Error::new)?;
    let analysis_id = Uuid::new_v4();
    sqlx::query("INSERT INTO ai_analyses(id,user_id,account_id,symbol,timeframe,market_data,analysis) VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(analysis_id).bind(user).bind(account).bind(&symbol).bind(&timeframe)
        .bind(serde_json::to_value(&quote)?).bind(serde_json::to_value(&analysis)?).execute(&state.db).await?;
    Ok(format_analysis(&analysis))
}

async fn create_intent(
    state: &AppState,
    telegram_id: i64,
    side: Side,
    args: &[&str],
) -> anyhow::Result<String> {
    anyhow::ensure!(
        args.len() == 4,
        "Format: /buy EURUSD 0.01 1.0700 1.0800 (symbol volume SL TP)"
    );
    let symbol = args[0].to_ascii_uppercase();
    validate_symbol_timeframe(&symbol, "H1")?;
    let volume: Decimal = args[1].parse()?;
    let sl: Decimal = args[2].parse()?;
    let tp: Decimal = args[3].parse()?;
    anyhow::ensure!(volume > Decimal::ZERO, "Volume harus lebih besar dari nol");
    let user = user_id(state, telegram_id).await?;
    let account = account(state, user).await?;
    let quote = state
        .mt5
        .quote(account, &symbol)
        .await
        .map_err(anyhow::Error::new)?;
    let entry = match side {
        Side::Buy => quote.ask,
        Side::Sell => quote.bid,
    };
    let side_text = match side {
        Side::Buy => "BUY",
        Side::Sell => "SELL",
    };
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO trade_intents(id,user_id,account_id,symbol,side,volume,entry_price,stop_loss,take_profit,status,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'PENDING_CONFIRMATION',now()+interval '5 minutes')")
        .bind(id).bind(user).bind(account).bind(&symbol).bind(side_text).bind(volume).bind(entry).bind(sl).bind(tp).execute(&state.db).await?;
    Ok(format!("INTENT:{id}\nCONFIRM TRADE (DEMO MOCK)\n{side_text} {symbol} | Lot {volume}\nEntry: {entry}\nSL: {sl}\nTP: {tp}\n\nBelum ada order yang dikirim. Intent kedaluwarsa dalam 5 menit."))
}

async fn confirm_intent(
    state: &AppState,
    telegram_id: i64,
    intent_id: Uuid,
) -> anyhow::Result<String> {
    let user = user_id(state, telegram_id).await?;
    let order = crate::trading::confirm(
        state,
        intent_id,
        ConfirmIntent {
            user_id: user,
            idempotency_key: format!("telegram:{telegram_id}:{intent_id}"),
        },
    )
    .await
    .map_err(anyhow::Error::new)?;
    Ok(format!("ORDER DEMO BERHASIL\nTicket: {}\nHarga eksekusi: {}\nStatus: {}\nVerifikasi ticket di MetaTrader 5.", order.ticket, order.executed_price, order.status))
}

async fn cancel_intent(
    state: &AppState,
    telegram_id: i64,
    intent_id: Uuid,
) -> anyhow::Result<String> {
    let user = user_id(state, telegram_id).await?;
    let changed = sqlx::query("UPDATE trade_intents SET status='CANCELLED',updated_at=now() WHERE id=$1 AND user_id=$2 AND status='PENDING_CONFIRMATION'")
        .bind(intent_id).bind(user).execute(&state.db).await?.rows_affected();
    anyhow::ensure!(
        changed == 1,
        "Intent sudah diproses, kedaluwarsa, atau tidak ditemukan"
    );
    Ok("Intent dibatalkan. Tidak ada order yang dikirim.".to_owned())
}

async fn stop_trading(state: &AppState, telegram_id: i64) -> anyhow::Result<&'static str> {
    let user = user_id(state, telegram_id).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE risk_profiles SET trading_enabled=false,updated_at=now() WHERE user_id=$1 AND account_id IS NULL")
        .bind(user).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO audit_logs(user_id,event_type,entity_type,entity_id,metadata) VALUES($1,'KILL_SWITCH_ACTIVATED','user',$1,'{}')")
        .bind(user).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok("TRADING STOPPED\nOrder BUY dan SELL baru diblokir. Analisis dan riwayat tetap tersedia.")
}

async fn resume_trading(
    state: &AppState,
    telegram_id: i64,
    callback_user: Uuid,
    issued_at: i64,
) -> anyhow::Result<String> {
    let user = user_id(state, telegram_id).await?;
    anyhow::ensure!(user == callback_user, "Konfirmasi bukan milik user ini");
    let age = Utc::now().timestamp() - issued_at;
    anyhow::ensure!((0..=300).contains(&age), "Konfirmasi sudah kedaluwarsa");
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE risk_profiles SET trading_enabled=true,updated_at=now() WHERE user_id=$1 AND account_id IS NULL")
        .bind(user).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO audit_logs(user_id,event_type,entity_type,entity_id,metadata) VALUES($1,'TRADING_RESUMED','user',$1,'{}')")
        .bind(user).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok("Trading intent DEMO diaktifkan. Live trading tetap nonaktif dan setiap order masih membutuhkan konfirmasi.".to_owned())
}

async fn history(state: &AppState, telegram_id: i64) -> anyhow::Result<String> {
    let user = user_id(state, telegram_id).await?;
    let rows = sqlx::query("SELECT symbol,side,volume,status,mt5_ticket,created_at FROM orders WHERE user_id=$1 ORDER BY created_at DESC LIMIT 10")
        .bind(user).fetch_all(&state.db).await?;
    if rows.is_empty() {
        return Ok("Belum ada riwayat order.".to_owned());
    }
    let mut out = String::from("10 ORDER TERAKHIR\n");
    for row in rows {
        let ticket: Option<i64> = row.try_get("mt5_ticket")?;
        out.push_str(&format!(
            "\n{} {} · {} lot\nStatus: {} · Ticket: {}\n",
            row.try_get::<String, _>("symbol")?,
            row.try_get::<String, _>("side")?,
            row.try_get::<Decimal, _>("volume")?,
            row.try_get::<String, _>("status")?,
            ticket.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
        ));
    }
    Ok(out)
}

fn validate_symbol_timeframe(symbol: &str, timeframe: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !symbol.is_empty()
            && symbol.len() <= 12
            && symbol
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '.'),
        "Symbol tidak valid"
    );
    anyhow::ensure!(
        matches!(timeframe, "M1" | "M5" | "M15" | "M30" | "H1" | "H4" | "D1"),
        "Timeframe tidak didukung"
    );
    Ok(())
}

fn format_analysis(a: &Analysis) -> String {
    format!("{} MARKET ANALYSIS\nTimeframe: {}\nBias: {}\nSignal: {}\nConfidence: {}%\nEntry: {}\nSL: {}\nTP: {}\n\n{}\n\nAI dapat salah dan tidak menjamin profit. Tidak ada order yang dibuat.", a.symbol, a.timeframe, a.market_bias, a.signal, a.confidence, a.entry, a.stop_loss.map(|v| v.to_string()).unwrap_or_else(|| "-".into()), a.take_profit.map(|v| v.to_string()).unwrap_or_else(|| "-".into()), a.reasoning_summary.join("\n"))
}

fn welcome() -> &'static str {
    "SELAMAT DATANG\n\nAI memberikan analisis pasar. Anda membuat setiap keputusan trading. Dana tetap berada di broker Anda.\n\nAkun MOCK DEMO telah dibuat. Jalankan /analyze EURUSD H1 untuk mencoba. Forex berisiko dan AI dapat salah."
}

fn help() -> &'static str {
    "MENU\n/start - Registrasi/reset akun mock\n/status - Status trading\n/analyze EURUSD H1 - Analisis\n/resumetrading - Aktifkan intent demo\n/buy EURUSD 0.01 1.0700 1.0800 - Buat intent BUY\n/sell EURUSD 0.01 1.0800 1.0700 - Buat intent SELL\n/history - Riwayat order\n/stoptrading - Emergency stop\n/security - Keamanan\n\nBUY/SELL tidak langsung mengeksekusi order. Tombol konfirmasi terpisah selalu diperlukan."
}

fn security() -> &'static str {
    "SECURITY\n✓ Dana tetap di broker\n✓ MT5 MOCK tidak memakai kredensial broker\n✓ AI tidak menerima password\n✓ Order memerlukan konfirmasi\n✓ Emergency stop tersedia\n\nTidak ada sistem yang sepenuhnya bebas risiko."
}

fn friendly_error(error: &anyhow::Error) -> String {
    let detail = if let Some(app_error) = error.downcast_ref::<AppError>() {
        app_error.to_string()
    } else if error.downcast_ref::<sqlx::Error>().is_some() {
        "Layanan database sedang tidak tersedia".to_owned()
    } else {
        error
            .chain()
            .next()
            .map(ToString::to_string)
            .unwrap_or_else(|| "Permintaan tidak dapat diproses".to_owned())
    };
    format!("Tidak dapat melanjutkan: {detail}\nTidak ada order baru yang dibuat.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_supported_market_request() {
        assert!(validate_symbol_timeframe("EURUSD", "H1").is_ok());
        assert!(validate_symbol_timeframe("XAUUSD", "M15").is_ok());
    }

    #[test]
    fn rejects_unsafe_symbol_or_timeframe() {
        assert!(validate_symbol_timeframe("EUR/USD", "H1").is_err());
        assert!(validate_symbol_timeframe("EURUSD", "H2").is_err());
    }
}
