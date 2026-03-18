"""POST /api/records endpoint - Return paginated records to Feishu."""

import logging
from typing import Any

import orjson
from fastapi import APIRouter, Depends
from fastapi.responses import ORJSONResponse

from src.adapters import registry
from src.config import settings
from src.middleware.error_handler import ConnectorError, wrap_adapter_exception
from src.middleware.signature import validate_request_signature
from src.models.response import RecordData, RecordsData
from src.utils.id_generator import make_field_id, make_primary_id
from src.utils.pagination import decode_page_token, encode_page_token
from src.utils.params_parser import parse_feishu_params

logger = logging.getLogger(__name__)
router = APIRouter()


@router.post("/api/records", response_class=ORJSONResponse)
async def records(
    body: bytes = Depends(validate_request_signature),
) -> dict[str, Any]:
    """Return paginated record data for the configured data source."""
    try:
        payload = orjson.loads(body)
    except orjson.JSONDecodeError as e:
        raise ConnectorError("UNKNOWN_ERROR", detail="Invalid JSON body") from e

    try:
        config, params = parse_feishu_params(payload)
    except Exception as e:
        raise ConnectorError(
            "CONNECTION_FAILED", detail="Failed to parse datasource config",
        ) from e

    adapter = registry.get_default()

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

    custom_sql = config.custom_sql if config.mode == "sql" else None

    try:
        rows = await adapter.fetch_records(
            config, offset, fetch_limit,
            schema_name=config.schema_name or "public",
            table_name=config.table_name,
            selected_fields=config.selected_fields,
            custom_sql=custom_sql,
        )
    except ConnectorError:
        raise
    except Exception as e:
        raise wrap_adapter_exception(e) from e

    has_more = len(rows) > max_page_size
    result_rows = rows[:max_page_size]

    col_types: dict[str, int] = {}
    col_field_ids: dict[str, str] = {}
    if result_rows:
        try:
            if custom_sql:
                sql_cols = await adapter.get_sql_columns(config, custom_sql)
                for c in sql_cols:
                    col_types[c.name] = adapter.map_field_type(c.data_type)
                    col_field_ids[c.name] = make_field_id(c.name)
            else:
                columns = await adapter.list_columns(
                    config,
                    config.schema_name or "public",
                    config.table_name or "",
                )
                for c in columns:
                    col_types[c.name] = adapter.map_field_type(c.data_type)
                    col_field_ids[c.name] = make_field_id(c.name)
        except Exception:
            logger.warning("Failed to get column types, defaulting to text", exc_info=True)
            for key in result_rows[0].keys():
                col_types[key] = 1
                col_field_ids[key] = make_field_id(key)

    pk_columns: list[str] = []
    if config.mode == "table" and config.table_name:
        try:
            pk_columns = await adapter.get_primary_key_columns(
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
            data[field_id] = adapter.format_value(row[col_name], field_type)

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
