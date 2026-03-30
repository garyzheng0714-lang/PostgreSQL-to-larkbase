"""Base adapter protocol and shared types for data source connectors.

All data source adapters must implement the DataSourceAdapter protocol.
Each adapter defines its own config type extending BaseConfig.

Architecture:
  DataSourceAdapter[C: BaseConfig]
  ├── test_connection(config) -> ConnectionResult
  ├── list_databases(config) -> list[str]
  ├── list_tables(config, database) -> list[TableInfo]
  ├── list_columns(config, ...) -> list[ColumnInfo]
  ├── fetch_records(config, ...) -> list[Any]
  ├── get_primary_key_columns(config, ...) -> list[str]
  ├── map_field_type(source_type) -> int
  ├── can_be_primary(field_type) -> bool
  └── format_value(value, field_type) -> Any
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Protocol, TypeVar, runtime_checkable

from pydantic import BaseModel


class BaseConfig(BaseModel):
    """Base configuration shared by all data source adapters."""

    host: str
    port: int
    username: str
    password: str
    database: str
    # SSL
    ssl_mode: str = "disable"
    ssl_root_cert: str | None = None
    ssl_cert: str | None = None
    ssl_key: str | None = None
    # Timeouts (None = use server defaults)
    connect_timeout: int | None = None
    query_timeout: int | None = None


C = TypeVar("C", bound=BaseConfig)


@dataclass
class ConnectionResult:
    """Result of a connection test."""

    success: bool
    message: str = ""
    server_version: str = ""
    database_size: str = ""
    table_count: int = 0


@dataclass
class TableInfo:
    """Metadata for a table or view."""

    name: str
    type: str  # "table" or "view"
    estimated_rows: int = 0


@dataclass
class ColumnInfo:
    """Metadata for a single column."""

    name: str
    data_type: str
    is_nullable: bool = True
    ordinal_position: int = 0


@runtime_checkable
class DataSourceAdapter(Protocol[C]):
    """Protocol that all data source adapters must implement."""

    source_type: str

    async def test_connection(self, config: C) -> ConnectionResult: ...

    async def list_databases(self, config: C) -> list[str]: ...

    async def list_schemas(self, config: C) -> list[str]: ...

    async def list_tables(
        self, config: C, schema_name: str = "public"
    ) -> list[TableInfo]: ...

    async def list_columns(
        self, config: C, schema_name: str, table_name: str
    ) -> list[ColumnInfo]: ...

    async def get_sql_columns(self, config: C, sql: str) -> list[ColumnInfo]: ...

    async def fetch_records(
        self, config: C, offset: int, limit: int,
        *, schema_name: str = "public", table_name: str | None = None,
        selected_fields: list[str] | None = None, custom_sql: str | None = None,
    ) -> list[Any]: ...

    async def get_primary_key_columns(
        self, config: C, schema_name: str, table_name: str
    ) -> list[str]: ...

    async def preview_sql(
        self, config: C, sql: str, limit: int = 10,
    ) -> list[Any]: ...

    async def validate_sql(self, config: C, sql: str) -> bool: ...

    def map_field_type(self, source_type: str) -> int: ...

    def can_be_primary(self, field_type: int) -> bool: ...

    def format_value(self, value: Any, field_type: int) -> Any: ...

    async def close(self) -> None: ...
