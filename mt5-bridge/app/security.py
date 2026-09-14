import hmac
import os
from fastapi import Header, HTTPException

def verify_internal_key(x_internal_api_key: str = Header(default="")) -> None:
    expected = os.environ.get("MT5_BRIDGE_API_KEY", "")
    if not expected or not hmac.compare_digest(expected, x_internal_api_key):
        raise HTTPException(status_code=401, detail="unauthorized")

