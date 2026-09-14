import os
import random
from datetime import datetime, timezone
from decimal import Decimal
from .models import OrderRequest, OrderResult, Quote

MOCK = os.environ.get("MT5_MODE", "MOCK").upper() == "MOCK"

def quote(symbol: str) -> Quote:
    if MOCK:
        mid = Decimal("1.07500") if symbol != "XAUUSD" else Decimal("2500.00")
        return Quote(symbol=symbol, bid=mid, ask=mid + Decimal("0.00020"), timestamp=datetime.now(timezone.utc))
    import MetaTrader5 as mt5
    tick = mt5.symbol_info_tick(symbol)
    if tick is None: raise RuntimeError("quote unavailable")
    return Quote(symbol=symbol, bid=Decimal(str(tick.bid)), ask=Decimal(str(tick.ask)), timestamp=datetime.fromtimestamp(tick.time, timezone.utc))

def order(request: OrderRequest) -> OrderResult:
    if MOCK:
        q = quote(request.symbol)
        price = q.ask if request.side == "BUY" else q.bid
        return OrderResult(ticket=random.SystemRandom().randint(100000000, 999999999), executed_price=price, status="OPEN")
    import MetaTrader5 as mt5
    q = quote(request.symbol); price = q.ask if request.side == "BUY" else q.bid
    kind = mt5.ORDER_TYPE_BUY if request.side == "BUY" else mt5.ORDER_TYPE_SELL
    result = mt5.order_send({"action": mt5.TRADE_ACTION_DEAL, "symbol": request.symbol, "volume": float(request.volume), "type": kind, "price": float(price), "sl": float(request.stop_loss or 0), "tp": float(request.take_profit or 0), "deviation": request.max_slippage_points, "magic": 260914, "comment": request.trade_intent_id[:24]})
    if result is None or result.retcode != mt5.TRADE_RETCODE_DONE: raise RuntimeError("broker rejected order")
    return OrderResult(ticket=result.order, executed_price=Decimal(str(result.price)), status="OPEN")

