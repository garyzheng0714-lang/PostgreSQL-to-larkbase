"""Pagination token encoding/decoding for Bitable records endpoint."""

import base64

import orjson


def encode_page_token(offset: int) -> str:
    """Encode an offset value into a base64 page token.

    Args:
        offset: The row offset for the next page.

    Returns:
        URL-safe base64 encoded token string.
    """
    payload = orjson.dumps({"offset": offset})
    return base64.urlsafe_b64encode(payload).decode().rstrip("=")


def decode_page_token(token: str) -> int:
    """Decode a page token back to an offset value.

    Args:
        token: Base64 encoded page token.

    Returns:
        Row offset integer.

    Raises:
        ValueError: If the token is invalid or malformed.
    """
    padded = token + "=" * (4 - len(token) % 4)
    try:
        data = orjson.loads(base64.urlsafe_b64decode(padded))
        return int(data["offset"])
    except (KeyError, ValueError, orjson.JSONDecodeError) as e:
        raise ValueError(f"Invalid page token: {token}") from e
