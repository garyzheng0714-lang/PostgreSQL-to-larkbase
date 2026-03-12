"""Bilingual error definitions for Feishu Bitable connector protocol."""

import orjson


def make_error_msg(zh: str, en: str) -> str:
    """Create a bilingual error message JSON string.

    Args:
        zh: Chinese error message.
        en: English error message.

    Returns:
        JSON string with both language versions.
    """
    return orjson.dumps({"zh": zh, "en": en}).decode()


# Feishu standard error codes
ERR_CONFIG = 1254400
ERR_AUTH = 1254403
ERR_SYSTEM = 1254500
ERR_RATE_LIMIT = 1254501
ERR_PAYMENT = 1254505

# Error message templates
ERRORS = {
    "CONNECTION_FAILED": (
        ERR_CONFIG,
        make_error_msg(
            "数据库连接失败，请检查连接信息",
            "Database connection failed, please check connection info",
        ),
    ),
    "QUERY_TIMEOUT": (
        ERR_SYSTEM,
        make_error_msg(
            "查询超时，请优化 SQL 或减少数据量",
            "Query timeout, please optimize SQL or reduce data volume",
        ),
    ),
    "INVALID_SQL": (
        ERR_CONFIG,
        make_error_msg("SQL 语法错误", "SQL syntax error"),
    ),
    "SIGNATURE_INVALID": (
        ERR_AUTH,
        make_error_msg("请求签名验证失败", "Request signature verification failed"),
    ),
    "TOO_MANY_FIELDS": (
        ERR_CONFIG,
        make_error_msg(
            "字段数超过上限(299)，请减少选择的字段",
            "Field count exceeds limit (299), please reduce selected fields",
        ),
    ),
    "TABLE_NOT_FOUND": (
        ERR_CONFIG,
        make_error_msg("指定的表不存在", "Specified table does not exist"),
    ),
    "PERMISSION_DENIED": (
        ERR_AUTH,
        make_error_msg(
            "数据库权限不足，请检查用户权限",
            "Insufficient database permissions, please check user privileges",
        ),
    ),
    "UNKNOWN_ERROR": (
        ERR_SYSTEM,
        make_error_msg("未知系统错误", "Unknown system error"),
    ),
}
