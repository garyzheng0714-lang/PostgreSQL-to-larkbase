"""DatasourceConfig model for user-saved sync configuration."""

import re

from pydantic import BaseModel, Field, field_validator

_IDENTIFIER_RE = re.compile(r"^[\w][\w ]*$", re.UNICODE)
_DANGEROUS_SQL_RE = re.compile(
    r"\b(INSERT|UPDATE|DELETE|DROP|ALTER|CREATE|TRUNCATE|GRANT|REVOKE|EXEC)\b",
    re.IGNORECASE,
)


class FieldRename(BaseModel):
    """Field rename mapping.

    Args:
        original: Original PostgreSQL column name.
        display: Display name in Bitable.
    """

    original: str
    display: str


class NumberFormat(BaseModel):
    """Number field format configuration.

    Args:
        precision: Decimal precision.
    """

    precision: int = 0


class DatasourceConfig(BaseModel):
    """Configuration saved by frontend and passed back by Feishu on each API call.

    Args:
        host: PostgreSQL server address.
        port: PostgreSQL server port.
        username: Database username.
        password: Database password.
        database: Target database name.
        mode: Query mode - 'table' or 'sql'.
        schema_name: PostgreSQL schema (table mode only).
        table_name: Table or view name (table mode only).
        selected_fields: List of field names to sync, None means all.
        custom_sql: Custom SQL query (sql mode only).
        field_renames: Field display name overrides.
        number_formats: Number field format configs.
        auto_sync: Whether to enable hourly auto-sync.
    """

    host: str
    port: int = Field(default=5432, ge=1, le=65535)
    username: str
    password: str
    database: str
    mode: str = Field(default="table", pattern="^(table|sql)$")
    schema_name: str = "public"
    table_name: str | None = None
    selected_fields: list[str] | None = None
    custom_sql: str | None = None
    field_renames: dict[str, str] | None = None
    number_formats: dict[str, NumberFormat] | None = None
    auto_sync: bool = True
    ssl_mode: str = Field(
        default="disable",
        pattern="^(disable|allow|prefer|require|verify-ca|verify-full)$",
    )
    ssl_root_cert: str | None = None
    ssl_cert: str | None = None
    ssl_key: str | None = None
    connect_timeout: int | None = Field(default=None, ge=1, le=60)
    query_timeout: int | None = Field(default=None, ge=1, le=300)

    @field_validator("schema_name")
    @classmethod
    def validate_schema_name(cls, v: str) -> str:
        if not _IDENTIFIER_RE.match(v):
            raise ValueError(f"Invalid schema name: {v}")
        return v

    @field_validator("table_name")
    @classmethod
    def validate_table_name(cls, v: str | None) -> str | None:
        if v is not None and not _IDENTIFIER_RE.match(v):
            raise ValueError(f"Invalid table name: {v}")
        return v

    @field_validator("selected_fields")
    @classmethod
    def validate_selected_fields(cls, v: list[str] | None) -> list[str] | None:
        if v is not None:
            for f in v:
                if not _IDENTIFIER_RE.match(f):
                    raise ValueError(f"Invalid field name: {f}")
        return v

    @field_validator("custom_sql")
    @classmethod
    def validate_custom_sql(cls, v: str | None) -> str | None:
        if v is not None and _DANGEROUS_SQL_RE.search(v):
            raise ValueError("SQL contains disallowed keywords (write operations)")
        return v
