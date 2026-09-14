from fastapi import Depends, FastAPI, HTTPException
from . import mt5_client
from .models import OrderRequest, OrderResult, Quote
from .security import verify_internal_key
from .sessions import registry

app = FastAPI(title="Internal MT5 Bridge", docs_url=None, redoc_url=None)

@app.get("/health")
async def health(): return {"status": "ok"}

@app.get("/accounts/{account_id}/quote/{symbol}", response_model=Quote, dependencies=[Depends(verify_internal_key)])
async def get_quote(account_id: str, symbol: str):
    worker = await registry.worker(account_id)
    async with worker.lock:
        try: return mt5_client.quote(symbol)
        except Exception as exc: raise HTTPException(503, "market data unavailable") from exc

@app.post("/accounts/{account_id}/orders", response_model=OrderResult, dependencies=[Depends(verify_internal_key)])
async def create_order(account_id: str, request: OrderRequest):
    if account_id != request.account_id: raise HTTPException(400, "account mismatch")
    worker = await registry.worker(account_id)
    async with worker.lock:
        try: return mt5_client.order(request)
        except Exception as exc: raise HTTPException(502, "order execution failed") from exc

