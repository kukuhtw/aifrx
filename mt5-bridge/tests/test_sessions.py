import os
import unittest
from unittest.mock import patch

from app.sessions import AccountSession


class AccountSessionTests(unittest.TestCase):
    def test_native_bridge_rejects_other_account(self):
        with patch.dict(os.environ, {"MT5_MODE": "DEMO", "MT5_ACCOUNT_ID": "account-a"}):
            session = AccountSession()
            session.require_account("account-a")
            with self.assertRaises(ValueError):
                session.require_account("account-b")
            with self.assertRaises(RuntimeError):
                session.require_verified()

    def test_mock_mode_accepts_demo_account_without_native_binding(self):
        with patch.dict(os.environ, {"MT5_MODE": "MOCK"}):
            AccountSession().require_account("mock-account")


if __name__ == "__main__":
    unittest.main()
