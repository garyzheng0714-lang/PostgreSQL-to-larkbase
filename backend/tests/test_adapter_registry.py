"""Tests for the adapter registry."""

import pytest

from src.adapters import registry
from src.adapters.postgres.service import PostgresAdapter


class TestAdapterRegistry:

    def setup_method(self) -> None:
        registry._registry.clear()

    def test_register_and_get(self) -> None:
        adapter = PostgresAdapter()
        registry.register(adapter)
        result = registry.get("postgres")
        assert result is adapter

    def test_get_unknown_raises_key_error(self) -> None:
        with pytest.raises(KeyError, match="mysql"):
            registry.get("mysql")

    def test_get_default_returns_postgres(self) -> None:
        adapter = PostgresAdapter()
        registry.register(adapter)
        result = registry.get_default()
        assert result is adapter

    def test_available_types(self) -> None:
        adapter = PostgresAdapter()
        registry.register(adapter)
        assert "postgres" in registry.available_types()

    def test_register_overwrites(self) -> None:
        adapter1 = PostgresAdapter()
        adapter2 = PostgresAdapter()
        registry.register(adapter1)
        registry.register(adapter2)
        assert registry.get("postgres") is adapter2
