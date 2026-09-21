import os
import random
from datetime import datetime, timezone
from decimal import Decimal
from .models import LoginRequest, OrderRequest, OrderResult, Quote, VerifiedAccount

MOCK = os.environ.get("MT5_MODE", "MOCK").upper() == "MOCK"


def verify(request: LoginRequest) -> VerifiedAccount:
    if MOCK:
        raise RuntimeError("mock bridge cannot verify broker accounts")
    import MetaTrader5 as mt5
    path = os.environ.get("MT5_TERMINAL_PATH")
    if not path:
        raise RuntimeError("MT5_TERMINAL_PATH is required")
    mt5.shutdown()
    if not mt5.initialize(path, login=request.login, password=request.password,
                          server=request.server, portable=True, timeout=10000):
        raise RuntimeError("terminal login failed")
    info = mt5.account_info()
    if info is None or info.login != request.login or info.server != request.server:
        mt5.shutdown()
        raise RuntimeError("broker account identity mismatch")
    if info.trade_mode == mt5.ACCOUNT_TRADE_MODE_DEMO:
        mode = "DEMO"
    elif info.trade_mode == mt5.ACCOUNT_TRADE_MODE_REAL:
        mode = "LIVE"
    else:
        mt5.shutdown()
        raise RuntimeError("unsupported broker account type")
    return VerifiedAccount(login=info.login, server=info.server, broker=info.company,
                           account_type=mode, balance=Decimal(str(info.balance)),
                           equity=Decimal(str(info.equity)))


def _check_identity(identity: tuple[int, str] | None) -> None:
    if MOCK:
        return
    import MetaTrader5 as mt5
    if identity is None:
        raise RuntimeError("account is not verified")
    info = mt5.account_info()
    if info is None or (info.login, info.server) != identity:
        raise RuntimeError("terminal account changed or disconnected")

def quote(symbol: str, identity: tuple[int, str] | None = None) -> Quote:
    if MOCK:
        mid = Decimal("1.07500") if symbol != "XAUUSD" else Decimal("2500.00")
        return Quote(symbol=symbol, bid=mid, ask=mid + Decimal("0.00020"), timestamp=datetime.now(timezone.utc))
    import MetaTrader5 as mt5
    _check_identity(identity)
    info = mt5.symbol_info(symbol)
    if info is None or (not info.visible and not mt5.symbol_select(symbol, True)):
        raise RuntimeError("symbol unavailable")
    tick = mt5.symbol_info_tick(symbol)
    if tick is None or tick.bid <= 0 or tick.ask <= 0: raise RuntimeError("quote unavailable")
    return Quote(symbol=symbol, bid=Decimal(str(tick.bid)), ask=Decimal(str(tick.ask)), timestamp=datetime.fromtimestamp(tick.time, timezone.utc))

def order(request: OrderRequest, identity: tuple[int, str] | None = None) -> OrderResult:
    if MOCK:
        q = quote(request.symbol)
        price = q.ask if request.side == "BUY" else q.bid
        return OrderResult(ticket=random.SystemRandom().randint(100000000, 999999999), executed_price=price, status="OPEN")
    import MetaTrader5 as mt5
    _check_identity(identity)
    info = mt5.account_info()
    terminal = mt5.terminal_info()
    if not info.trade_allowed or terminal is None or not terminal.trade_allowed:
        raise RuntimeError("trading is disabled in MT5")
    symbol = mt5.symbol_info(request.symbol)
    if symbol is None or request.volume < Decimal(str(symbol.volume_min)) or request.volume > Decimal(str(symbol.volume_max)):
        raise RuntimeError("invalid broker volume")
    step = Decimal(str(symbol.volume_step))
    if step <= 0 or (request.volume - Decimal(str(symbol.volume_min))) % step != 0:
        raise RuntimeError("invalid broker volume step")
    q = quote(request.symbol, identity)
    if (datetime.now(timezone.utc) - q.timestamp).total_seconds() > 30:
        raise RuntimeError("quote is stale")
    price = q.ask if request.side == "BUY" else q.bid
    kind = mt5.ORDER_TYPE_BUY if request.side == "BUY" else mt5.ORDER_TYPE_SELL
    filling = mt5.ORDER_FILLING_FOK if symbol.filling_mode & 1 else mt5.ORDER_FILLING_IOC if symbol.filling_mode & 2 else None
    if filling is None:
        raise RuntimeError("unsupported symbol filling mode")
    payload = {"action": mt5.TRADE_ACTION_DEAL, "symbol": request.symbol, "volume": float(request.volume), "type": kind, "price": float(price), "sl": float(request.stop_loss or 0), "tp": float(request.take_profit or 0), "deviation": request.max_slippage_points, "magic": 260914, "comment": request.trade_intent_id[:24], "type_filling": filling}
    checked = mt5.order_check(payload)
    if checked is None or checked.retcode != 0:
        raise RuntimeError("broker preflight rejected order")
    result = mt5.order_send(payload)
    if result is None or result.retcode != mt5.TRADE_RETCODE_DONE: raise RuntimeError("broker rejected order")
    return OrderResult(ticket=result.order, executed_price=Decimal(str(result.price)), status="OPEN")
