"""Adapter registry for looking up data source adapters by type."""

from __future__ import annotations

import logging
from typing import Any

from src.adapters.base import DataSourceAdapter

logger = logging.getLogger(__name__)

_registry: dict[str, DataSourceAdapter[Any]] = {}


def register(adapter: DataSourceAdapter[Any]) -> None:
    """Register a data source adapter.

    Args:
        adapter: Adapter instance to register.
    """
    _registry[adapter.source_type] = adapter
    logger.info("Registered adapter: %s", adapter.source_type)


def get(source_type: str) -> DataSourceAdapter[Any]:
    """Get a registered adapter by source type.

    Args:
        source_type: Data source type identifier (e.g., "postgres").

    Returns:
        The registered adapter.

    Raises:
        KeyError: If no adapter is registered for the given type.
    """
    adapter = _registry.get(source_type)
    if adapter is None:
        available = list(_registry.keys())
        raise KeyError(
            f"No adapter registered for source type '{source_type}'. "
            f"Available: {available}"
        )
    return adapter


def get_default() -> DataSourceAdapter[Any]:
    """Get the default adapter (postgres).

    Returns:
        The postgres adapter.

    Raises:
        KeyError: If no postgres adapter is registered.
    """
    return get("postgres")


def available_types() -> list[str]:
    """List all registered adapter types."""
    return list(_registry.keys())


async def close_all() -> None:
    """Close all registered adapters and release resources."""
    for adapter in _registry.values():
        await adapter.close()
    logger.info("All adapters closed")
