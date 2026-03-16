"""SHA-1 request signature verification for Feishu Base connector protocol.

Note: SHA-1 is used because the Feishu Bitable connector protocol mandates it.
This is not a voluntary choice — the protocol specification requires
SHA1(timestamp + nonce + secretKey + body).hex() for request signing.
"""

import hashlib
import hmac
import logging
import time

from fastapi import Request

from src.config import settings

SIGNATURE_MAX_AGE_SECONDS = 300

logger = logging.getLogger(__name__)


class ConnectorAuthError(Exception):
    """Raised when request signature verification fails.

    Args:
        error_key: Key into ERRORS dict for bilingual error message.
    """

    def __init__(self, error_key: str = "SIGNATURE_INVALID") -> None:
        self.error_key = error_key
        super().__init__(error_key)


def verify_signature(
    timestamp: str,
    nonce: str,
    secret_key: str,
    body: bytes,
    signature: str,
) -> bool:
    """Verify Feishu Base request signature using SHA-1.

    The signature is computed as: SHA1(timestamp + nonce + secretKey + body).hex()

    Args:
        timestamp: Value from X-Base-Request-Timestamp header.
        nonce: Value from X-Base-Request-Nonce header.
        secret_key: Pre-shared secret key.
        body: Raw request body bytes.
        signature: Expected signature from X-Base-Signature header.

    Returns:
        True if signature matches, False otherwise.
    """
    raw = f"{timestamp}{nonce}{secret_key}".encode() + body
    computed = hashlib.sha1(raw).hexdigest()
    return hmac.compare_digest(computed, signature)


async def validate_request_signature(request: Request) -> bytes:
    """FastAPI dependency that validates request signature.

    Reads the raw body, verifies the SHA-1 signature from headers,
    and returns the body bytes for further processing.

    In development mode (secret_key == "testBase"), signature validation
    is skipped to allow local testing without Feishu headers.

    Args:
        request: The incoming FastAPI request.

    Returns:
        Raw request body bytes.

    Raises:
        HTTPException: 403 if signature is present but invalid.
    """
    body = await request.body()
    logger.info(
        "Incoming %s %s (body=%d bytes)",
        request.method, request.url.path, len(body),
    )
    timestamp = request.headers.get("X-Base-Request-Timestamp", "")
    nonce = request.headers.get("X-Base-Request-Nonce", "")
    signature = request.headers.get("X-Base-Signature", "")

    if not all([timestamp, nonce, signature]):
        if settings.secret_key == "testBase":
            return body
        logger.warning(
            "Missing signature headers: ts=%r nonce=%r sig=%r",
            bool(timestamp), bool(nonce), bool(signature),
        )
        raise ConnectorAuthError("SIGNATURE_INVALID")

    try:
        request_time = int(timestamp)
        if abs(time.time() - request_time) > SIGNATURE_MAX_AGE_SECONDS:
            logger.warning("Request timestamp expired: %s", timestamp)
            raise ConnectorAuthError("SIGNATURE_INVALID")
    except ValueError:
        logger.warning("Invalid timestamp format: %s", timestamp)
        raise ConnectorAuthError("SIGNATURE_INVALID")

    is_valid = verify_signature(
        timestamp=timestamp,
        nonce=nonce,
        secret_key=settings.secret_key,
        body=body,
        signature=signature,
    )

    if not is_valid:
        logger.warning(
            "Invalid signature for %s %s",
            request.method, request.url.path,
        )
        raise ConnectorAuthError("SIGNATURE_INVALID")

    logger.info("Signature verified for %s %s", request.method, request.url.path)
    return body
