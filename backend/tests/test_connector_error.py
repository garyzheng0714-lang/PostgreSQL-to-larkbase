"""Tests for the unified error handling system."""

import asyncpg
import pytest

from src.middleware.error_handler import (
    ConnectorError,
    wrap_adapter_exception,
)


class TestConnectorError:

    def test_error_key_only(self) -> None:
        err = ConnectorError("CONNECTION_FAILED")
        assert err.error_key == "CONNECTION_FAILED"
        assert err.detail == ""
        assert "CONNECTION_FAILED" in str(err)

    def test_error_with_detail(self) -> None:
        err = ConnectorError("QUERY_TIMEOUT", detail="took 30s")
        assert err.error_key == "QUERY_TIMEOUT"
        assert err.detail == "took 30s"
        assert "took 30s" in str(err)


class TestWrapAdapterException:

    def test_connector_error_passes_through(self) -> None:
        original = ConnectorError("INVALID_SQL")
        result = wrap_adapter_exception(original)
        assert result is original

    def test_timeout_error(self) -> None:
        result = wrap_adapter_exception(TimeoutError("query timeout"))
        assert result.error_key == "QUERY_TIMEOUT"

    def test_os_error(self) -> None:
        result = wrap_adapter_exception(OSError("connection refused"))
        assert result.error_key == "CONNECTION_FAILED"

    def test_unknown_error(self) -> None:
        result = wrap_adapter_exception(RuntimeError("unexpected"))
        assert result.error_key == "UNKNOWN_ERROR"
        assert "unexpected" in result.detail
