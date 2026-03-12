"""DatasourceConfig model for user-saved sync configuration."""

from pydantic import BaseModel, Field


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
