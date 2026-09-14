import asyncio
from dataclasses import dataclass

@dataclass
class AccountWorker:
    account_id: str
    lock: asyncio.Lock

class SessionRegistry:
    """Serializes each account. Production Windows deployment should assign one terminal process per worker."""
    def __init__(self) -> None:
        self._guard = asyncio.Lock()
        self._workers: dict[str, AccountWorker] = {}

    async def worker(self, account_id: str) -> AccountWorker:
        async with self._guard:
            return self._workers.setdefault(account_id, AccountWorker(account_id, asyncio.Lock()))

registry = SessionRegistry()

