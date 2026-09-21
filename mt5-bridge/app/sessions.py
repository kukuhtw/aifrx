import asyncio
import os


class AccountSession:
    """One bridge process owns one terminal and one account."""

    def __init__(self) -> None:
        self.account_id = os.environ.get("MT5_ACCOUNT_ID", "")
        self.lock = asyncio.Lock()
        self.login: int | None = None
        self.server: str | None = None

    def require_account(self, account_id: str) -> None:
        if os.environ.get("MT5_MODE", "MOCK").upper() == "MOCK":
            return
        if not self.account_id or account_id != self.account_id:
            raise ValueError("account is not assigned to this bridge")

    def require_verified(self) -> tuple[int, str]:
        if self.login is None or self.server is None:
            raise RuntimeError("account is not verified in this bridge process")
        return self.login, self.server


session = AccountSession()
