"""POST /api/table_meta endpoint - Return table schema to Feishu."""

import logging
from typing import Any

import asyncpg
import orjson
from fastapi import APIRouter, Depends
from fastapi.responses import ORJSONResponse

from src.middleware.signature import validate_request_signature
from src.models.error import ERRORS
from src.models.response import FieldMeta, FieldProperty, TableMetaData
from src.services import pg_service
from src.services.type_mapper import can_be_primary, map_pg_type
from src.utils.id_generator import make_field_id
from src.utils.params_parser import parse_feishu_params

logger = logging.getLogger(__name__)
router = APIRouter()


def _build_error_response(error_key: str) -> dict[str, Any]:
    """Build a standardized error response dict."""
    code, msg = ERRORS.get(error_key, ERRORS["UNKNOWN_ERROR"])
    return {"code": code, "msg": msg, "data": None}


@router.post("/api/table_meta", response_class=ORJSONResponse)
async def table_meta(
    body: bytes = Depends(validate_request_signature),
) -> dict[str, Any]:
    """Return table schema (field definitions) for the configured data source.

    Feishu Base server calls this endpoint with the saved datasourceConfig
    to get the table structure before creating or updating the sync table.

    Args:
        body: Validated request body bytes from signature verification.

    Returns:
        Table schema response with field definitions.
    """
    try:
        payload = orjson.loads(body)
    except orjson.JSONDecodeError:
        return _build_error_response("UNKNOWN_ERROR")

    try:
        config, params = parse_feishu_params(payload)
    except Exception:
        logger.exception("Failed to parse datasource config")
        return _build_error_response("CONNECTION_FAILED")

    try:
        if config.mode == "sql" and config.custom_sql:
            columns = await pg_service.get_sql_columns(
                config, config.custom_sql
            )
            table_name = "SQL Query Result"
        else:
            columns = await pg_service.list_columns(
                config,
                config.schema_name or "public",
                config.table_name or "",
            )
            table_name = config.table_name or "Untitled"
    except asyncpg.InvalidCatalogNameError:
        return _build_error_response("TABLE_NOT_FOUND")
    except asyncpg.InsufficientPrivilegeError:
        return _build_error_response("PERMISSION_DENIED")
    except TimeoutError:
        return _build_error_response("QUERY_TIMEOUT")
    except Exception:
        logger.exception("Failed to fetch table metadata")
        return _build_error_response("CONNECTION_FAILED")

    if not columns:
        return _build_error_response("TABLE_NOT_FOUND")

    if config.selected_fields is not None:
        selected = set(config.selected_fields)
        columns = [c for c in columns if c["name"] in selected]

    if len(columns) > 299:
        return _build_error_response("TOO_MANY_FIELDS")

    renames = config.field_renames or {}
    pk_columns: list[str] = []
    if config.mode == "table" and config.table_name:
        try:
            pk_columns = await pg_service.get_primary_key_columns(
                config,
                config.schema_name or "public",
                config.table_name,
            )
        except Exception:
            logger.warning("Failed to get primary key columns", exc_info=True)

    fields: list[FieldMeta] = []
    primary_set = False
    for col in columns:
        col_name = col["name"]
        data_type = col.get("data_type", col.get("udt_name", "text"))
        field_type = map_pg_type(data_type)
        field_id = make_field_id(col_name)
        display_name = renames.get(col_name, col_name)

        is_primary = False
        if not primary_set:
            if pk_columns and col_name in pk_columns:
                if can_be_primary(field_type):
                    is_primary = True
                    primary_set = True
            elif not pk_columns and can_be_primary(field_type):
                is_primary = True
                primary_set = True

        prop = None
        if config.number_formats and col_name in config.number_formats:
            fmt = config.number_formats[col_name]
            prop = FieldProperty(formatter=f"0.{'0' * fmt.precision}")

        fields.append(
            FieldMeta(
                fieldID=field_id,
                fieldName=display_name,
                fieldType=field_type,
                isPrimary=is_primary,
                description=f"PostgreSQL: {data_type}",
                property=prop,
            )
        )

    if not primary_set and fields:
        for f in fields:
            if can_be_primary(f.fieldType):
                f.isPrimary = True
                break

    table_name_clean = table_name.replace("/", "").replace("\\", "")
    table_name_clean = table_name_clean.replace("?", "").replace("*", "")
    table_name_clean = table_name_clean.replace("[", "").replace("]", "")
    table_name_clean = table_name_clean.replace(":", "")[:100]

    data = TableMetaData(tableName=table_name_clean, fields=fields)
    return {"code": 0, "msg": "", "data": data.model_dump()}
