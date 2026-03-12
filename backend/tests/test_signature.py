"""Tests for SHA-1 request signature verification."""

import hashlib

from src.middleware.signature import verify_signature


class TestVerifySignature:
    """Test suite for verify_signature function."""

    def test_valid_signature(self) -> None:
        """Verify that a correctly computed signature passes."""
        timestamp = "1700000000"
        nonce = "abc123"
        secret_key = "testBase"
        body = b'{"params":"{}", "context":"{}"}'

        raw = f"{timestamp}{nonce}{secret_key}".encode() + body
        expected_sig = hashlib.sha1(raw).hexdigest()

        assert verify_signature(
            timestamp, nonce, secret_key, body, expected_sig
        )

    def test_invalid_signature(self) -> None:
        """Verify that an incorrect signature fails."""
        assert not verify_signature(
            "1700000000",
            "abc123",
            "testBase",
            b'{"params":"{}"}',
            "invalid_signature_value",
        )

    def test_empty_body(self) -> None:
        """Verify signature works with empty body."""
        timestamp = "1700000000"
        nonce = "xyz"
        secret_key = "key"
        body = b""

        raw = f"{timestamp}{nonce}{secret_key}".encode()
        expected_sig = hashlib.sha1(raw).hexdigest()

        assert verify_signature(
            timestamp, nonce, secret_key, body, expected_sig
        )

    def test_different_secret_key_fails(self) -> None:
        """Verify that using a wrong secret key produces a mismatch."""
        timestamp = "1700000000"
        nonce = "abc"
        body = b"test"

        raw = f"{timestamp}{nonce}correct_key".encode() + body
        sig = hashlib.sha1(raw).hexdigest()

        assert not verify_signature(
            timestamp, nonce, "wrong_key", body, sig
        )
