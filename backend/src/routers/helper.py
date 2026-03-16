"""Frontend helper API endpoints for configuration UI."""

import logging
from typing import Any

import asyncpg
from fastapi import APIRouter, Depends, Header, HTTPException
from fastapi.responses import ORJSONResponse

from src.config import settings
from src.models.datasource import DatasourceConfig
from src.models.request import (
    HelperColumnsRequest,
    HelperConnectionRequest,
    HelperSQLPreviewRequest,
    HelperTablesRequest,
)
from src.services import pg_service
from src.services.type_mapper import map_pg_type

logger = logging.getLogger(__name__)


async def verify_helper_api_key(
    x_helper_api_key: str = Header(default=""),
) -> None:
    """Verify API key for helper endpoints (skipped in dev mode)."""
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


def _make_ds_config(req: HelperConnectionRequest) -> DatasourceConfig:
    """Convert a helper request to DatasourceConfig."""
    return DatasourceConfig(
        host=req.host,
        port=req.port,
        username=req.username,
        password=req.password,
        database=req.database,
    )


@router.post("/test_connection", response_class=ORJSONResponse)
async def test_connection(req: HelperConnectionRequest) -> dict[str, Any]:
    """Test PostgreSQL connection with provided credentials.

    Args:
        req: Connection parameters.

    Returns:
        Success status with optional error message.
    """
    config = _make_ds_config(req)
    try:
        await pg_service.test_connection(config)
        return {"success": True, "message": ""}
    except asyncpg.InvalidPasswordError:
        return {
            "success": False,
            "message": "用户名或密码错误 / Invalid username or password",
        }
    except asyncpg.InvalidCatalogNameError:
        return {
            "success": False,
            "message": (
                f"数据库 '{req.database}' 不存在 / "
                f"Database '{req.database}' does not exist"
            ),
        }
    except OSError:
        return {
            "success": False,
            "message": (
                "无法连接到服务器，请检查地址和端口 / "
                "Cannot connect, check host and port"
            ),
        }
    except Exception:
        logger.exception("Connection test failed")
        return {"success": False, "message": "连接失败 / Connection failed"}


@router.post("/databases", response_class=ORJSONResponse)
async def list_databases(req: HelperConnectionRequest) -> dict[str, Any]:
    """List all accessible databases.

    Args:
        req: Connection parameters.

    Returns:
        List of database names.
    """
    config = _make_ds_config(req)
    try:
        databases = await pg_service.list_databases(config)
        return {"success": True, "data": databases}
    except Exception:
        logger.exception("Failed to list databases")
        return {"success": False, "message": "获取数据库列表失败", "data": []}


@router.post("/schemas", response_class=ORJSONResponse)
async def list_schemas(req: HelperConnectionRequest) -> dict[str, Any]:
    """List all schemas in the connected database.

    Args:
        req: Connection parameters.

    Returns:
        List of schema names.
    """
    config = _make_ds_config(req)
    try:
        schemas = await pg_service.list_schemas(config)
        return {"success": True, "data": schemas}
    except Exception:
        logger.exception("Failed to list schemas")
        return {"success": False, "message": "获取 Schema 列表失败", "data": []}


@router.post("/tables", response_class=ORJSONResponse)
async def list_tables(req: HelperTablesRequest) -> dict[str, Any]:
    """List all tables and views in a schema.

    Args:
        req: Connection parameters with schema name.

    Returns:
        List of table/view objects with name and type.
    """
    config = _make_ds_config(req)
    try:
        tables = await pg_service.list_tables(config, req.schema_name)
        return {"success": True, "data": tables}
    except Exception:
        logger.exception("Failed to list tables")
        return {"success": False, "message": "获取表列表失败", "data": []}


@router.post("/columns", response_class=ORJSONResponse)
async def list_columns(req: HelperColumnsRequest) -> dict[str, Any]:
    """List all columns in a table with type information.

    Args:
        req: Connection parameters with schema and table name.

    Returns:
        List of column objects with name, type, and mapped Bitable type.
    """
    config = _make_ds_config(req)
    try:
        columns = await pg_service.list_columns(
            config, req.schema_name, req.table_name
        )
        enriched = []
        for col in columns:
            bitable_type = map_pg_type(col["data_type"])
            enriched.append({**col, "bitable_type": bitable_type})
        return {"success": True, "data": enriched}
    except Exception:
        logger.exception("Failed to list columns")
        return {"success": False, "message": "获取列信息失败", "data": []}


@router.post("/preview_sql", response_class=ORJSONResponse)
async def preview_sql(req: HelperSQLPreviewRequest) -> dict[str, Any]:
    """Preview results of a custom SQL query (limited to 10 rows).

    Args:
        req: Connection parameters with SQL query.

    Returns:
        Preview data with columns and rows.
    """
    config = _make_ds_config(req)
    try:
        sql_cols = await pg_service.get_sql_columns(config, req.sql)
        columns = [
            {"name": c["name"], "data_type": c["data_type"]}
            for c in sql_cols
        ]

        rows = await pg_service.preview_sql(config, req.sql, limit=10)
        preview_rows = [dict(row) for row in rows]
        for row in preview_rows:
            for key, val in row.items():
                if not isinstance(val, (str, int, float, bool, type(None))):
                    row[key] = str(val)

        return {
            "success": True,
            "data": {"columns": columns, "rows": preview_rows},
        }
    except asyncpg.PostgresSyntaxError as e:
        return {"success": False, "message": f"SQL 语法错误: {e}"}
    except Exception:
        logger.exception("SQL preview failed")
        return {"success": False, "message": "SQL 预览失败"}
