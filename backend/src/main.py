"""FastAPI application entry point for PostgreSQL-to-Larkbase connector."""

import logging

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import ORJSONResponse

from src.config import settings
from src.middleware.signature import ConnectorAuthError
from src.models.error import ERRORS
from src.routers import helper, meta, records, table_meta

logging.basicConfig(
    level=getattr(logging, settings.log_level.upper(), logging.INFO),
    format="%(asctime)s %(levelname)s %(name)s: %(message)s",
)

app = FastAPI(
    title="PostgreSQL to Larkbase Connector",
    version="0.1.0",
    default_response_class=ORJSONResponse,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(meta.router)
app.include_router(table_meta.router)
app.include_router(records.router)
app.include_router(helper.router)


@app.exception_handler(ConnectorAuthError)
async def connector_auth_error_handler(
    _request: Request,
    exc: ConnectorAuthError,
) -> ORJSONResponse:
    """Return HTTP 200 with connector error code for auth failures.

    Feishu Base server expects all responses as HTTP 200 with error
    information in the response body, not as HTTP 4xx status codes.
    """
    code, msg = ERRORS.get(exc.error_key, ERRORS["SIGNATURE_INVALID"])
    return ORJSONResponse(
        status_code=200,
        content={"code": code, "msg": msg, "data": None},
    )


@app.get("/health")
async def health() -> dict[str, str]:
    """Health check endpoint.

    Returns:
        Status indicator.
    """
    return {"status": "ok"}
