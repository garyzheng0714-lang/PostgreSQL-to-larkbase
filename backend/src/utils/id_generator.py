"""Generate safe IDs conforming to Bitable fieldID and primaryID constraints."""

import hashlib
import re

_UNSAFE_CHARS = re.compile(r"[^a-zA-Z0-9_]")


def make_field_id(column_name: str) -> str:
    """Generate a valid Bitable fieldID from a PostgreSQL column name.

    Uses MD5 hash with 'fld_' prefix to guarantee uniqueness and avoid
    any Bitable reserved words. Max 20 chars, always safe.

    Args:
        column_name: Original PostgreSQL column name.

    Returns:
        Safe fieldID string (e.g. 'fld_a1b2c3d4e5f6').
    """
    digest = hashlib.md5(column_name.encode()).hexdigest()[:16]
    return f"fld_{digest}"


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
