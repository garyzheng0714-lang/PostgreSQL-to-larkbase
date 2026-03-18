"""Format PostgreSQL field values to Bitable-compatible types."""

import datetime
from decimal import Decimal
from typing import Any

from src.adapters.postgres.type_mapper import (
    FIELD_CHECKBOX,
    FIELD_CURRENCY,
    FIELD_DATE,
    FIELD_NUMBER,
)


def format_value(value: Any, field_type: int) -> Any:
    """Convert a PostgreSQL value to the format expected by Bitable.

    Returns:
        Formatted value. Falls back to str() for unrecognized types
        to prevent TypeError crashes.
    """
    if value is None:
        return None

    try:
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
    except (TypeError, ValueError, OverflowError):
        return str(value)


def _format_text(value: Any) -> str:
    if isinstance(value, (list, dict)):
        import orjson

        return orjson.dumps(value).decode()
    return str(value)


def _format_number(value: Any) -> float | int | None:
    if isinstance(value, Decimal):
        if value.is_nan() or value.is_infinite():
            return None
        if value == value.to_integral_value():
            return int(value)
        return float(value)
    if isinstance(value, (float, int)):
        return value
    return float(value)


def _format_date(value: Any) -> int | None:
    if isinstance(value, datetime.datetime):
        return int(value.timestamp() * 1000)
    if isinstance(value, datetime.date):
        dt = datetime.datetime.combine(
            value, datetime.time.min, tzinfo=datetime.UTC
        )
        return int(dt.timestamp() * 1000)
    return None


def _format_currency(value: Any) -> float | None:
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
