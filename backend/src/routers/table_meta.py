"""POST /api/table_meta endpoint - Return table schema to Feishu."""

import logging
from typing import Any

import orjson
from fastapi import APIRouter, Depends
from fastapi.responses import ORJSONResponse

from src.adapters import registry
from src.middleware.error_handler import ConnectorError, wrap_adapter_exception
from src.middleware.signature import validate_request_signature
from src.models.response import FieldMeta, FieldProperty, TableMetaData
from src.utils.id_generator import make_field_id
from src.utils.params_parser import parse_feishu_params

logger = logging.getLogger(__name__)
router = APIRouter()


@router.post("/api/table_meta", response_class=ORJSONResponse)
async def table_meta(
    body: bytes = Depends(validate_request_signature),
) -> dict[str, Any]:
    """Return table schema (field definitions) for the configured data source."""
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

    try:
        if config.mode == "sql" and config.custom_sql:
            columns = await adapter.get_sql_columns(config, config.custom_sql)
            table_name = "SQL Query Result"
        else:
            columns = await adapter.list_columns(
                config,
                config.schema_name or "public",
                config.table_name or "",
            )
            table_name = config.table_name or "Untitled"
    except ConnectorError:
        raise
    except Exception as e:
        raise wrap_adapter_exception(e) from e

    if not columns:
        raise ConnectorError("TABLE_NOT_FOUND")

    if config.selected_fields is not None:
        selected = set(config.selected_fields)
        columns = [c for c in columns if c.name in selected]

    if len(columns) > 299:
        raise ConnectorError("TOO_MANY_FIELDS")

    renames = config.field_renames or {}
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

    fields: list[FieldMeta] = []
    primary_set = False
    for col in columns:
        col_name = col.name
        data_type = col.data_type
        field_type = adapter.map_field_type(data_type)
        field_id = make_field_id(col_name)
        display_name = renames.get(col_name, col_name)

        is_primary = False
        if not primary_set:
            if pk_columns and col_name in pk_columns:
                if adapter.can_be_primary(field_type):
                    is_primary = True
                    primary_set = True
            elif not pk_columns and adapter.can_be_primary(field_type):
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
            if adapter.can_be_primary(f.fieldType):
                f.isPrimary = True
                break

    table_name_clean = table_name.replace("/", "").replace("\\", "")
    table_name_clean = table_name_clean.replace("?", "").replace("*", "")
    table_name_clean = table_name_clean.replace("[", "").replace("]", "")
    table_name_clean = table_name_clean.replace(":", "")[:100]

    data = TableMetaData(tableName=table_name_clean, fields=fields)
    return {"code": 0, "msg": "", "data": data.model_dump()}
