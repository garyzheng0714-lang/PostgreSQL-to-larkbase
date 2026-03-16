"""Format PostgreSQL field values to Bitable-compatible types."""

import datetime
from decimal import Decimal
from typing import Any

from src.services.type_mapper import (
    FIELD_CHECKBOX,
    FIELD_CURRENCY,
    FIELD_DATE,
    FIELD_NUMBER,
)


def format_value(value: Any, field_type: int) -> Any:
    """Convert a PostgreSQL value to the format expected by Bitable.

    Args:
        value: Raw value from PostgreSQL query result.
        field_type: Bitable field type integer.

    Returns:
        Formatted value matching Bitable protocol requirements:
        - Text (1): str
        - Number (2): float or int
        - Date (5): Unix timestamp in milliseconds (int)
        - Checkbox (7): bool
        - Currency (8): float
        - All others: str
    """
    if value is None:
        return None

    match field_type:
        case 1 | 3 | 4 | 6 | 9 | 10 | 13:
            return _format_text(value)
        case x if x == FIELD_NUMBER:
            return _format_number(value)
        case x if x == FIELD_DATE:
            return _format_date(value)
        case x if x == FIELD_CHECKBOX:
            return bool(value)
        case x if x == FIELD_CURRENCY:
            return _format_currency(value)
        case _:
            return str(value)


def _format_text(value: Any) -> str:
    """Convert any value to string representation."""
    if isinstance(value, (list, dict)):
        import orjson

        return orjson.dumps(value).decode()
    return str(value)


def _format_number(value: Any) -> float | int | None:
    """Convert numeric value, handling Decimal properly."""
    if isinstance(value, Decimal):
        if value.is_nan() or value.is_infinite():
            return None
        if value == value.to_integral_value():
            return int(value)
        return float(value)
    if isinstance(value, float):
        return value
    if isinstance(value, int):
        return value
    return float(value)


def _format_date(value: Any) -> int | None:
    """Convert date/datetime to Unix milliseconds timestamp."""
    if isinstance(value, datetime.datetime):
        return int(value.timestamp() * 1000)
    if isinstance(value, datetime.date):
        dt = datetime.datetime.combine(
            value, datetime.time.min, tzinfo=datetime.UTC
        )
        return int(dt.timestamp() * 1000)
    return None


def _format_currency(value: Any) -> float | None:
    """Convert currency value, stripping symbols from PG money type."""
    try:
        if isinstance(value, str):
            cleaned = value.replace("$", "").replace(",", "").strip()
            return float(cleaned)
        if isinstance(value, Decimal):
            if value.is_nan() or value.is_infinite():
                return None
            return float(value)
        return float(value)
    except (ValueError, TypeError):
        return None
