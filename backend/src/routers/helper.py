"""Frontend helper API endpoints for configuration UI."""

import logging
from typing import Any

from fastapi import APIRouter, Depends, Header, HTTPException
from fastapi.responses import ORJSONResponse

from src.adapters import registry
from src.adapters.postgres.service import PostgresConfig
from src.config import settings
from src.models.request import (
    HelperColumnsRequest,
    HelperConnectionRequest,
    HelperSQLPreviewRequest,
    HelperTablesRequest,
)

logger = logging.getLogger(__name__)


async def verify_helper_api_key(
    x_helper_api_key: str = Header(default=""),
) -> None:
    if settings.is_dev_mode:
        return
    if not settings.helper_api_key:
        return
    if x_helper_api_key != settings.helper_api_key:
        raise HTTPException(status_code=401, detail="Invalid API key")


router = APIRouter(
    prefix="/api/helper",
    tags=["helper"],
    dependencies=[Depends(verify_helper_api_key)],
)


def _make_config(req: HelperConnectionRequest) -> PostgresConfig:
    return PostgresConfig(
        host=req.host,
        port=req.port,
        username=req.username,
        password=req.password,
        database=req.database,
        ssl_mode=req.ssl_mode,
        ssl_root_cert=req.ssl_root_cert,
        ssl_cert=req.ssl_cert,
        ssl_key=req.ssl_key,
        connect_timeout=req.connect_timeout,
        query_timeout=req.query_timeout,
    )


@router.post("/test_connection", response_class=ORJSONResponse)
async def test_connection(req: HelperConnectionRequest) -> dict[str, Any]:
    adapter = registry.get_default()
    config = _make_config(req)
    result = await adapter.test_connection(config)
    return {
        "success": result.success,
        "message": result.message,
        "server_version": result.server_version,
        "database_size": result.database_size,
        "table_count": result.table_count,
    }


@router.post("/databases", response_class=ORJSONResponse)
async def list_databases(req: HelperConnectionRequest) -> dict[str, Any]:
    adapter = registry.get_default()
    config = _make_config(req)
    try:
        databases = await adapter.list_databases(config)
        return {"success": True, "data": databases}
    except Exception:
        logger.exception("Failed to list databases")
        return {"success": False, "message": "获取数据库列表失败", "data": []}


@router.post("/schemas", response_class=ORJSONResponse)
async def list_schemas(req: HelperConnectionRequest) -> dict[str, Any]:
    adapter = registry.get_default()
    config = _make_config(req)
    try:
        schemas = await adapter.list_schemas(config)
        return {"success": True, "data": schemas}
    except Exception:
        logger.exception("Failed to list schemas")
        return {"success": False, "message": "获取 Schema 列表失败", "data": []}


@router.post("/tables", response_class=ORJSONResponse)
async def list_tables(req: HelperTablesRequest) -> dict[str, Any]:
    adapter = registry.get_default()
    config = _make_config(req)
    try:
        tables = await adapter.list_tables(config, req.schema_name)
        return {
            "success": True,
            "data": [
                {
                    "name": t.name,
                    "type": t.type,
                    "estimated_rows": t.estimated_rows,
                }
                for t in tables
            ],
        }
    except Exception:
        logger.exception("Failed to list tables")
        return {"success": False, "message": "获取表列表失败", "data": []}


@router.post("/columns", response_class=ORJSONResponse)
async def list_columns(req: HelperColumnsRequest) -> dict[str, Any]:
    adapter = registry.get_default()
    config = _make_config(req)
    try:
        columns = await adapter.list_columns(
            config, req.schema_name, req.table_name,
        )
        enriched = []
        for col in columns:
            bitable_type = adapter.map_field_type(col.data_type)
            enriched.append({
                "name": col.name,
                "data_type": col.data_type,
                "is_nullable": col.is_nullable,
                "ordinal_position": col.ordinal_position,
                "bitable_type": bitable_type,
            })
        return {"success": True, "data": enriched}
    except Exception:
        logger.exception("Failed to list columns")
        return {"success": False, "message": "获取列信息失败", "data": []}


@router.post("/preview_sql", response_class=ORJSONResponse)
async def preview_sql(req: HelperSQLPreviewRequest) -> dict[str, Any]:
    adapter = registry.get_default()
    config = _make_config(req)
    try:
        sql_cols = await adapter.get_sql_columns(config, req.sql)
        columns = [
            {"name": c.name, "data_type": c.data_type}
            for c in sql_cols
        ]

        rows = await adapter.preview_sql(config, req.sql, limit=10)
        preview_rows = [dict(row) for row in rows]
        for row in preview_rows:
            for key, val in row.items():
                if not isinstance(val, (str, int, float, bool, type(None))):
                    row[key] = str(val)

        return {
            "success": True,
            "data": {"columns": columns, "rows": preview_rows},
        }
    except Exception as e:
        logger.exception("SQL preview failed")
        msg = f"SQL 语法错误: {e}" if "syntax" in str(e).lower() else "SQL 预览失败"
        return {"success": False, "message": msg}
