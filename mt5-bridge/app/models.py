from datetime import datetime
from decimal import Decimal
from typing import Literal
from pydantic import BaseModel, Field

class LoginRequest(BaseModel):
    account_id: str
    login: int
    password: str = Field(repr=False)
    server: str

class VerifiedAccount(BaseModel):
    login: int
    server: str
    broker: str
    account_type: Literal["DEMO", "LIVE"]
    balance: Decimal
    equity: Decimal

class OrderRequest(BaseModel):
    trade_intent_id: str
    account_id: str
    symbol: str = Field(pattern=r"^[A-Z0-9.]{1,12}$")
    side: Literal["BUY", "SELL"]
    volume: Decimal = Field(gt=0)
    stop_loss: Decimal | None = None
    take_profit: Decimal | None = None
    max_slippage_points: int = Field(ge=0, le=1000)

class Quote(BaseModel):
    symbol: str
    bid: Decimal
    ask: Decimal
    timestamp: datetime

class OrderResult(BaseModel):
    ticket: int
    executed_price: Decimal
    status: str
