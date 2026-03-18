"""FastAPI application entry point for the data connector platform."""

import logging
from contextlib import asynccontextmanager
from collections.abc import AsyncGenerator

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import ORJSONResponse

from src.adapters import registry
from src.adapters.postgres.pool import get_pool_manager
from src.adapters.postgres.service import PostgresAdapter
from src.config import settings
from src.middleware.error_handler import register_error_handlers
from src.middleware.signature import ConnectorAuthError
from src.models.error import ERRORS
from src.routers import helper, meta, records, table_meta

logging.basicConfig(
    level=getattr(logging, settings.log_level.upper(), logging.INFO),
    format="%(asctime)s %(levelname)s %(name)s: %(message)s",
)


@asynccontextmanager
async def lifespan(_app: FastAPI) -> AsyncGenerator[None, None]:
    registry.register(PostgresAdapter())
    pm = get_pool_manager()
    await pm.start_cleanup_loop()
    yield
    await registry.close_all()


app = FastAPI(
    title="Data Connector Platform",
    version="1.3.0",
    default_response_class=ORJSONResponse,
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins,
    allow_credentials=True,
    allow_methods=["GET", "POST", "OPTIONS"],
    allow_headers=[
        "Content-Type",
        "X-Base-Request-Timestamp",
        "X-Base-Request-Nonce",
        "X-Base-Signature",
        "X-Helper-Api-Key",
    ],
)

register_error_handlers(app)

app.include_router(meta.router)
app.include_router(table_meta.router)
app.include_router(records.router)
app.include_router(helper.router)


@app.exception_handler(ConnectorAuthError)
async def connector_auth_error_handler(
    _request: Request,
    exc: ConnectorAuthError,
) -> ORJSONResponse:
    code, msg = ERRORS.get(exc.error_key, ERRORS["SIGNATURE_INVALID"])
    return ORJSONResponse(
        status_code=200,
        content={"code": code, "msg": msg, "data": None},
    )


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}
