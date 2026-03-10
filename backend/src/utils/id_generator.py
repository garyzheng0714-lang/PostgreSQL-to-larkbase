"""Generate safe IDs conforming to Bitable fieldID and primaryID constraints."""

import hashlib
import re

_UNSAFE_CHARS = re.compile(r"[^a-zA-Z0-9_]")


def make_field_id(column_name: str) -> str:
    """Generate a valid Bitable fieldID from a PostgreSQL column name.

    Rules: max 50 chars, only English letters, numbers, underscore.

    Args:
        column_name: Original PostgreSQL column name.

    Returns:
        Safe fieldID string.
    """
    sanitized = _UNSAFE_CHARS.sub("_", column_name)
    if not sanitized or not sanitized[0].isalpha():
        sanitized = f"f_{sanitized}"
    sanitized = sanitized.strip("_")
    if not sanitized:
        sanitized = f"f_{hashlib.md5(column_name.encode()).hexdigest()[:8]}"
    if len(sanitized) > 50:
        prefix = sanitized[:42]
        suffix = hashlib.md5(column_name.encode()).hexdigest()[:7]
        sanitized = f"{prefix}_{suffix}"
    return sanitized


def make_primary_id(row_pk: object) -> str:
    """Generate a valid Bitable primaryID from a row's primary key value.

    Rules: max 100 chars, only English letters, numbers, underscore.

    Args:
        row_pk: Primary key value or row number.

    Returns:
        Safe primaryID string.
    """
    raw = str(row_pk)
    sanitized = _UNSAFE_CHARS.sub("_", raw)
    if not sanitized:
        sanitized = f"row_{hashlib.md5(raw.encode()).hexdigest()[:12]}"
    return sanitized[:100]
