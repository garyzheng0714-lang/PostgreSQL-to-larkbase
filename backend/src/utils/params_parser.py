"""Parse Feishu connector request params robustly.

Feishu sends params in various nesting formats:
- params as JSON string: '{"datasourceConfig":"{\\"host\\":...}"}'
- params as dict: {"datasourceConfig": "{\\"host\\":...}"}
- datasourceConfig may be string or dict at each level
"""

import logging
from typing import Any

import orjson

from src.models.datasource import DatasourceConfig

logger = logging.getLogger(__name__)


def _safe_json_loads(val: Any) -> Any:
    """Parse JSON if string/bytes, return as-is if dict/list."""
    if isinstance(val, (str, bytes)):
        return orjson.loads(val)
    return val


def parse_feishu_params(payload: dict[str, Any]) -> tuple[DatasourceConfig, dict[str, Any]]:
    """Extract DatasourceConfig and extra params from Feishu request payload.

    Args:
        payload: Parsed JSON body from Feishu request.

    Returns:
        Tuple of (DatasourceConfig, full params dict).

    Raises:
        ValueError: If config cannot be parsed.
    """
    raw_params = payload.get("params", "{}")
    params = _safe_json_loads(raw_params)
    if not isinstance(params, dict):
        raise ValueError(f"params is not a dict: {type(params)}")

    logger.info("Parsed params keys: %s", list(params.keys()))

    ds_raw = params.get("datasourceConfig", "{}")
    logger.info("datasourceConfig raw type: %s", type(ds_raw).__name__)
    ds_data = _safe_json_loads(ds_raw)

    if isinstance(ds_data, dict) and "datasourceConfig" in ds_data:
        logger.info("Unwrapping nested datasourceConfig")
        inner = ds_data["datasourceConfig"]
        ds_data = _safe_json_loads(inner)

    if not isinstance(ds_data, dict):
        raise ValueError(f"datasourceConfig is not a dict: {type(ds_data)}")

    safe_keys = list(ds_data.keys())
    logger.info("Parsed datasourceConfig keys: %s", safe_keys)
    logger.info(
        "Config: host=%s database=%s mode=%s table=%s",
        ds_data.get("host", "?"),
        ds_data.get("database", "?"),
        ds_data.get("mode", "?"),
        ds_data.get("table_name", "?"),
    )

    config = DatasourceConfig(**ds_data)

    _log_request_context(payload)

    return config, params


def _log_request_context(payload: dict[str, Any]) -> None:
    """Log tenant/user info from the Feishu request context field."""
    raw_ctx = payload.get("context")
    if not raw_ctx:
        return
    try:
        ctx = _safe_json_loads(raw_ctx)
        if not isinstance(ctx, dict):
            return
        tenant = ctx.get("tenantKey", "?")
        log_id = ctx.get("bitable", {}).get("logID", "?")
        user_id = ctx.get("scriptArgs", {}).get("baseOpenID", "?")
        biz_id = ctx.get("bizInstanceID", "?")
        logger.info(
            "Context: tenant=%s logID=%s user=%s bizInstance=%s",
            tenant, log_id, user_id, biz_id,
        )
    except Exception:
        logger.debug("Failed to parse request context", exc_info=True)
