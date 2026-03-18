"""Unified error handling for the connector service.

All adapter-layer errors are converted to ConnectorError, which is
caught by a global FastAPI exception handler and returned in Feishu
protocol format (HTTP 200 with error code in body).

Error flow:
  adapter raises Exception
     ↓
  ConnectorError (or auto-wrapped)
     ↓
  FastAPI exception_handler
     ↓
  HTTP 200 {"code": X, "msg": "...", "data": null}
"""

import logging
from typing import Any

import asyncpg
from fastapi import FastAPI, Request
from fastapi.responses import ORJSONResponse

from src.models.error import ERRORS

logger = logging.getLogger(__name__)


class ConnectorError(Exception):
    """Unified error for all connector operations.

    Args:
        error_key: Key into the ERRORS dict (e.g., "CONNECTION_FAILED").
        detail: Optional detail message for logging (not shown to user).
    """

    def __init__(self, error_key: str, *, detail: str = "") -> None:
        self.error_key = error_key
        self.detail = detail
        super().__init__(f"{error_key}: {detail}" if detail else error_key)


_ASYNCPG_ERROR_MAP: dict[type[Exception], str] = {
    asyncpg.InvalidCatalogNameError: "TABLE_NOT_FOUND",
    asyncpg.InsufficientPrivilegeError: "PERMISSION_DENIED",
    asyncpg.PostgresSyntaxError: "INVALID_SQL",
    asyncpg.InvalidPasswordError: "CONNECTION_FAILED",
}


def wrap_adapter_exception(exc: Exception) -> ConnectorError:
    """Convert a raw adapter exception to a ConnectorError."""
    if isinstance(exc, ConnectorError):
        return exc

    if isinstance(exc, TimeoutError):
        return ConnectorError("QUERY_TIMEOUT", detail=str(exc))

    for exc_type, error_key in _ASYNCPG_ERROR_MAP.items():
        if isinstance(exc, exc_type):
            return ConnectorError(error_key, detail=str(exc))

    if isinstance(exc, asyncpg.PostgresError):
        return ConnectorError("CONNECTION_FAILED", detail=str(exc))

    if isinstance(exc, OSError):
        return ConnectorError("CONNECTION_FAILED", detail=str(exc))

    return ConnectorError("UNKNOWN_ERROR", detail=str(exc))


def _build_error_response(error_key: str) -> dict[str, Any]:
    code, msg = ERRORS.get(error_key, ERRORS["UNKNOWN_ERROR"])
    return {"code": code, "msg": msg, "data": None}


def register_error_handlers(app: FastAPI) -> None:
    """Register global exception handlers on the FastAPI app."""

    @app.exception_handler(ConnectorError)
    async def connector_error_handler(
        _request: Request, exc: ConnectorError,
    ) -> ORJSONResponse:
        if exc.detail:
            logger.warning(
                "ConnectorError: %s — %s", exc.error_key, exc.detail,
            )
        return ORJSONResponse(
            status_code=200,
            content=_build_error_response(exc.error_key),
        )
