"""Tests for fieldID and primaryID generation."""

from src.utils.id_generator import make_field_id, make_primary_id


class TestMakeFieldId:
    """Test suite for make_field_id function."""

    def test_returns_fld_prefix(self) -> None:
        """Verify output always starts with fld_ prefix."""
        assert make_field_id("user_name").startswith("fld_")

    def test_consistent_output(self) -> None:
        """Verify same input produces same output."""
        assert make_field_id("user_name") == make_field_id("user_name")

    def test_different_names_produce_different_ids(self) -> None:
        """Verify different column names produce different IDs."""
        assert make_field_id("id") != make_field_id("name")

    def test_max_length_20(self) -> None:
        """Verify output is at most 20 characters (fld_ + 16 hex)."""
        long_name = "a" * 100
        result = make_field_id(long_name)
        assert len(result) <= 20

    def test_safe_characters_only(self) -> None:
        """Verify output contains only safe characters."""
        result = make_field_id("用户名@special.chars")
        assert all(c.isalnum() or c == "_" for c in result)

    def test_empty_name(self) -> None:
        """Verify empty names produce valid output."""
        result = make_field_id("")
        assert len(result) > 0
        assert result.startswith("fld_")


class TestMakePrimaryId:
    """Test suite for make_primary_id function."""

    def test_numeric_pk(self) -> None:
        """Verify numeric primary keys work."""
        assert make_primary_id(42) == "42"

    def test_uuid_pk(self) -> None:
        """Verify UUID primary keys have hyphens replaced."""
        result = make_primary_id("550e8400-e29b-41d4-a716-446655440000")
        assert "-" not in result

    def test_max_length_100(self) -> None:
        """Verify output is truncated to 100 characters."""
        long_pk = "x" * 200
        result = make_primary_id(long_pk)
        assert len(result) <= 100

    def test_special_chars_replaced(self) -> None:
        """Verify only alphanumeric and underscore remain."""
        result = make_primary_id("pk/with:special@chars")
        assert all(c.isalnum() or c == "_" for c in result)
