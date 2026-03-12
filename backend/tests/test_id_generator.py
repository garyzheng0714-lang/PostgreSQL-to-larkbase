"""Tests for fieldID and primaryID generation."""

from src.utils.id_generator import make_field_id, make_primary_id


class TestMakeFieldId:
    """Test suite for make_field_id function."""

    def test_simple_ascii_name(self) -> None:
        """Verify simple ASCII column names pass through."""
        assert make_field_id("user_name") == "user_name"

    def test_chinese_characters_replaced(self) -> None:
        """Verify non-ASCII characters are replaced with underscores."""
        result = make_field_id("用户名")
        assert all(c.isalnum() or c == "_" for c in result)

    def test_starts_with_number_gets_prefix(self) -> None:
        """Verify names starting with numbers get 'f_' prefix."""
        result = make_field_id("123abc")
        assert result.startswith("f_")

    def test_max_length_50(self) -> None:
        """Verify output is truncated to 50 characters."""
        long_name = "a" * 100
        result = make_field_id(long_name)
        assert len(result) <= 50

    def test_special_chars_stripped(self) -> None:
        """Verify special characters are replaced."""
        result = make_field_id("col-name.with@special")
        assert "-" not in result
        assert "." not in result
        assert "@" not in result

    def test_empty_name(self) -> None:
        """Verify empty names produce valid output."""
        result = make_field_id("")
        assert len(result) > 0
        assert result[0].isalpha()


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
