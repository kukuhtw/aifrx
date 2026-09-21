import asyncio

from fastapi import Depends, FastAPI, HTTPException
from . import mt5_client
from .models import LoginRequest, OrderRequest, OrderResult, Quote, VerifiedAccount
from .security import verify_internal_key
from .sessions import session

app = FastAPI(title="Internal MT5 Bridge", docs_url=None, redoc_url=None)


@app.get("/health")
async def health():
    return {"status": "ok", "mode": "MOCK" if mt5_client.MOCK else "NATIVE"}


@app.post("/accounts/{account_id}/verify", response_model=VerifiedAccount, dependencies=[Depends(verify_internal_key)])
async def verify_account(account_id: str, request: LoginRequest):
    if account_id != request.account_id:
        raise HTTPException(400, "account mismatch")
    try:
        session.require_account(account_id)
    except ValueError as exc:
        raise HTTPException(404, str(exc)) from exc
    async with session.lock:
        try:
            result = await asyncio.to_thread(mt5_client.verify, request)
        except Exception as exc:
            raise HTTPException(422, "broker account verification failed") from exc
        session.login, session.server = result.login, result.server
        return result


@app.get("/accounts/{account_id}/quote/{symbol}", response_model=Quote, dependencies=[Depends(verify_internal_key)])
async def get_quote(account_id: str, symbol: str):
    try:
        session.require_account(account_id)
    except ValueError as exc:
        raise HTTPException(404, str(exc)) from exc
    async with session.lock:
        try:
            identity = None if mt5_client.MOCK else session.require_verified()
            return await asyncio.to_thread(mt5_client.quote, symbol, identity)
        except Exception as exc:
            raise HTTPException(503, "market data unavailable") from exc


@app.post("/accounts/{account_id}/orders", response_model=OrderResult, dependencies=[Depends(verify_internal_key)])
async def create_order(account_id: str, request: OrderRequest):
    if account_id != request.account_id:
        raise HTTPException(400, "account mismatch")
    try:
        session.require_account(account_id)
    except ValueError as exc:
        raise HTTPException(404, str(exc)) from exc
    async with session.lock:
        try:
            identity = None if mt5_client.MOCK else session.require_verified()
            return await asyncio.to_thread(mt5_client.order, request, identity)
        except Exception as exc:
            raise HTTPException(502, "order execution failed; check MT5 before retrying") from exc
