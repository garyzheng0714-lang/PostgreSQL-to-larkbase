"""POST /api/records endpoint - Return paginated records to Feishu."""

import logging
from typing import Any

import asyncpg
import orjson
from fastapi import APIRouter, Depends
from fastapi.responses import ORJSONResponse

from src.config import settings
from src.middleware.signature import validate_request_signature
from src.models.datasource import DatasourceConfig
from src.models.error import ERRORS
from src.models.response import RecordData, RecordsData
from src.services import pg_service
from src.services.field_formatter import format_value
from src.services.type_mapper import map_pg_type
from src.utils.id_generator import make_field_id, make_primary_id
from src.utils.pagination import decode_page_token, encode_page_token

logger = logging.getLogger(__name__)
router = APIRouter()


def _build_error_response(error_key: str) -> dict[str, Any]:
    """Build a standardized error response dict."""
    code, msg = ERRORS.get(error_key, ERRORS["UNKNOWN_ERROR"])
    return {"code": code, "msg": msg, "data": None}


@router.post("/api/records", response_class=ORJSONResponse)
async def records(
    body: bytes = Depends(validate_request_signature),
) -> dict[str, Any]:
    """Return paginated record data for the configured data source.

    Feishu Base server calls this endpoint repeatedly to fetch all records
    in pages of up to maxPageSize (max 1000) per request.

    Args:
        body: Validated request body bytes from signature verification.

    Returns:
        Records response with data and pagination token.
    """
    try:
        payload = orjson.loads(body)
    except orjson.JSONDecodeError:
        return _build_error_response("UNKNOWN_ERROR")

    try:
        params = orjson.loads(payload.get("params", "{}"))
        ds_config_raw = params.get("datasourceConfig", "{}")
        if isinstance(ds_config_raw, str):
            ds_config_data = orjson.loads(ds_config_raw)
        else:
            ds_config_data = ds_config_raw
        config = DatasourceConfig(**ds_config_data)
    except Exception:
        logger.exception("Failed to parse datasource config")
        return _build_error_response("CONNECTION_FAILED")

    max_page_size = min(params.get("maxPageSize", 1000), 1000)
    page_token = params.get("pageToken", "")

    offset = 0
    if page_token:
        try:
            offset = decode_page_token(page_token)
        except ValueError:
            offset = 0

    if offset >= settings.max_row_limit:
        empty_data = RecordsData(hasMore=False, records=[])
        return {"code": 0, "msg": "", "data": empty_data.model_dump()}

    remaining = settings.max_row_limit - offset
    fetch_limit = min(max_page_size + 1, remaining + 1)

    try:
        rows = await pg_service.fetch_records(config, offset, fetch_limit)
    except asyncpg.InvalidCatalogNameError:
        return _build_error_response("TABLE_NOT_FOUND")
    except asyncpg.InsufficientPrivilegeError:
        return _build_error_response("PERMISSION_DENIED")
    except TimeoutError:
        return _build_error_response("QUERY_TIMEOUT")
    except asyncpg.PostgresSyntaxError:
        return _build_error_response("INVALID_SQL")
    except Exception:
        logger.exception("Failed to fetch records")
        return _build_error_response("CONNECTION_FAILED")

    has_more = len(rows) > max_page_size
    result_rows = rows[:max_page_size]

    # Build column metadata for type mapping
    col_types: dict[str, int] = {}
    col_field_ids: dict[str, str] = {}
    if result_rows:
        if config.mode == "sql" and config.custom_sql:
            try:
                sql_cols = await pg_service.get_sql_columns(
                    config, config.custom_sql
                )
                for c in sql_cols:
                    col_types[c["name"]] = map_pg_type(c["data_type"])
                    col_field_ids[c["name"]] = make_field_id(c["name"])
            except Exception:
                logger.warning("Failed to get SQL column types", exc_info=True)
                for key in result_rows[0].keys():
                    col_types[key] = 1
                    col_field_ids[key] = make_field_id(key)
        else:
            try:
                columns = await pg_service.list_columns(
                    config,
                    config.schema_name or "public",
                    config.table_name or "",
                )
                for c in columns:
                    col_types[c["name"]] = map_pg_type(c["data_type"])
                    col_field_ids[c["name"]] = make_field_id(c["name"])
            except Exception:
                logger.warning("Failed to get column types", exc_info=True)
                for key in result_rows[0].keys():
                    col_types[key] = 1
                    col_field_ids[key] = make_field_id(key)

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

    record_list: list[RecordData] = []
    for idx, row in enumerate(result_rows):
        if pk_columns:
            pk_parts = [str(row.get(pk, "")) for pk in pk_columns]
            primary_id = make_primary_id("_".join(pk_parts))
        else:
            primary_id = make_primary_id(str(offset + idx + 1))

        data: dict[str, Any] = {}
        for col_name in row.keys():
            field_id = col_field_ids.get(col_name, make_field_id(col_name))
            field_type = col_types.get(col_name, 1)
            data[field_id] = format_value(row[col_name], field_type)

        record_list.append(RecordData(primaryID=primary_id, data=data))

    next_token = ""
    if has_more:
        next_offset = offset + max_page_size
        if next_offset < settings.max_row_limit:
            next_token = encode_page_token(next_offset)
        else:
            has_more = False

    result = RecordsData(
        hasMore=has_more,
        nextPageToken=next_token,
        records=record_list,
    )
    return {"code": 0, "msg": "", "data": result.model_dump()}
