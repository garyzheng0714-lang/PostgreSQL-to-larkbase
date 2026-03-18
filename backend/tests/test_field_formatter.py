"""Tests for field value formatting."""

import datetime
from decimal import Decimal

from src.adapters.postgres.formatter import format_value


class TestFormatValue:
    """Test suite for format_value function."""

    def test_none_returns_none(self) -> None:
        """Verify None input returns None for any type."""
        assert format_value(None, 1) is None
        assert format_value(None, 2) is None
        assert format_value(None, 5) is None

    def test_text_type(self) -> None:
        """Verify text values are converted to strings."""
        assert format_value("hello", 1) == "hello"
        assert format_value(123, 1) == "123"

    def test_number_int(self) -> None:
        """Verify integer values pass through."""
        assert format_value(42, 2) == 42

    def test_number_float(self) -> None:
        """Verify float values pass through."""
        assert format_value(3.14, 2) == 3.14

    def test_number_decimal_integer(self) -> None:
        """Verify Decimal integers convert to int."""
        result = format_value(Decimal("100"), 2)
        assert result == 100
        assert isinstance(result, int)

    def test_number_decimal_float(self) -> None:
        """Verify Decimal floats convert to float."""
        result = format_value(Decimal("3.14"), 2)
        assert result == 3.14
        assert isinstance(result, float)

    def test_date_datetime(self) -> None:
        """Verify datetime converts to Unix milliseconds."""
        dt = datetime.datetime(2024, 1, 1, 0, 0, 0,
                               tzinfo=datetime.UTC)
        result = format_value(dt, 5)
        assert result == 1704067200000

    def test_date_date_only(self) -> None:
        """Verify date-only converts to Unix milliseconds (UTC midnight)."""
        d = datetime.date(2024, 1, 1)
        result = format_value(d, 5)
        assert result == 1704067200000

    def test_checkbox_true(self) -> None:
        """Verify truthy values convert to True."""
        assert format_value(True, 7) is True
        assert format_value(1, 7) is True

    def test_checkbox_false(self) -> None:
        """Verify falsy values convert to False."""
        assert format_value(False, 7) is False
        assert format_value(0, 7) is False

    def test_currency_string(self) -> None:
        """Verify PG money string format is parsed."""
        result = format_value("$1,234.56", 8)
        assert result == 1234.56

    def test_currency_decimal(self) -> None:
        """Verify Decimal currency converts to float."""
        result = format_value(Decimal("99.99"), 8)
        assert result == 99.99

    def test_list_to_text(self) -> None:
        """Verify list values are JSON-serialized to text."""
        result = format_value(["a", "b"], 1)
        assert isinstance(result, str)
        assert "a" in result
