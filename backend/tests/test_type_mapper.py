"""Tests for PostgreSQL type to Bitable field type mapping."""

import pytest

from src.services.type_mapper import (
    FIELD_CHECKBOX,
    FIELD_CURRENCY,
    FIELD_DATE,
    FIELD_NUMBER,
    FIELD_TEXT,
    can_be_primary,
    map_pg_type,
)


class TestMapPgType:
    """Test suite for map_pg_type function."""

    @pytest.mark.parametrize(
        "pg_type",
        ["text", "varchar", "character varying", "char", "uuid", "json",
         "jsonb", "xml", "bytea", "inet", "cidr"],
    )
    def test_text_types(self, pg_type: str) -> None:
        """Verify text-like PG types map to FIELD_TEXT."""
        assert map_pg_type(pg_type) == FIELD_TEXT

    @pytest.mark.parametrize(
        "pg_type",
        ["int4", "integer", "int8", "bigint", "float4", "float8",
         "numeric", "decimal", "serial", "smallint", "real",
         "double precision"],
    )
    def test_numeric_types(self, pg_type: str) -> None:
        """Verify numeric PG types map to FIELD_NUMBER."""
        assert map_pg_type(pg_type) == FIELD_NUMBER

    @pytest.mark.parametrize(
        "pg_type", ["bool", "boolean"]
    )
    def test_boolean_types(self, pg_type: str) -> None:
        """Verify boolean PG types map to FIELD_CHECKBOX."""
        assert map_pg_type(pg_type) == FIELD_CHECKBOX

    @pytest.mark.parametrize(
        "pg_type",
        ["date", "timestamp", "timestamptz",
         "timestamp without time zone",
         "timestamp with time zone"],
    )
    def test_date_types(self, pg_type: str) -> None:
        """Verify date/timestamp PG types map to FIELD_DATE."""
        assert map_pg_type(pg_type) == FIELD_DATE

    def test_money_type(self) -> None:
        """Verify money PG type maps to FIELD_CURRENCY."""
        assert map_pg_type("money") == FIELD_CURRENCY

    def test_unknown_type_defaults_to_text(self) -> None:
        """Verify unknown types fall back to FIELD_TEXT."""
        assert map_pg_type("some_custom_type") == FIELD_TEXT

    def test_array_type_maps_to_text(self) -> None:
        """Verify array types map to FIELD_TEXT."""
        assert map_pg_type("int4[]") == FIELD_TEXT
        assert map_pg_type("text[]") == FIELD_TEXT

    def test_parameterized_type(self) -> None:
        """Verify types with params like varchar(255) are handled."""
        assert map_pg_type("varchar(255)") == FIELD_TEXT
        assert map_pg_type("numeric(10,2)") == FIELD_NUMBER

    def test_case_insensitive(self) -> None:
        """Verify type matching is case-insensitive."""
        assert map_pg_type("TEXT") == FIELD_TEXT
        assert map_pg_type("Integer") == FIELD_NUMBER

    def test_whitespace_handling(self) -> None:
        """Verify leading/trailing whitespace is stripped."""
        assert map_pg_type("  text  ") == FIELD_TEXT


class TestCanBePrimary:
    """Test suite for can_be_primary function."""

    def test_allowed_primary_types(self) -> None:
        """Verify types that can be primary columns."""
        assert can_be_primary(FIELD_TEXT)
        assert can_be_primary(FIELD_NUMBER)
        assert can_be_primary(FIELD_DATE)
        assert can_be_primary(FIELD_CURRENCY)

    def test_disallowed_primary_types(self) -> None:
        """Verify types that cannot be primary columns."""
        assert not can_be_primary(FIELD_CHECKBOX)
        assert not can_be_primary(3)  # Select
        assert not can_be_primary(4)  # MultiSelect
