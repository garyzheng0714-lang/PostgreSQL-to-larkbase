"""PostgreSQL data source adapter implementation."""

from __future__ import annotations

import logging
from typing import Any

import asyncpg

from src.adapters.base import (
    BaseConfig,
    ColumnInfo,
    ConnectionResult,
    DataSourceAdapter,
    TableInfo,
)
from src.adapters.postgres.formatter import format_value
from src.adapters.postgres.pool import get_pool_manager
from src.adapters.postgres.type_mapper import can_be_primary, map_pg_type
from src.middleware.error_handler import ConnectorError

logger = logging.getLogger(__name__)


class PostgresConfig(BaseConfig):
    """PostgreSQL-specific configuration."""

    schema_name: str = "public"


class PostgresAdapter:
    """PostgreSQL adapter implementing the DataSourceAdapter protocol."""

    source_type: str = "postgres"

    async def test_connection(self, config: PostgresConfig) -> ConnectionResult:
        pm = get_pool_manager()
        try:
            async with pm.acquire(
                config.host, config.port, config.username,
                config.password, config.database,
            ) as conn:
                version = await conn.fetchval("SELECT version()")
                size_row = await conn.fetchval(
                    "SELECT pg_size_pretty(pg_database_size(current_database()))"
                )
                table_count = await conn.fetchval(
                    "SELECT count(*) FROM information_schema.tables "
                    "WHERE table_schema NOT IN "
                    "('pg_catalog', 'information_schema', 'pg_toast')"
                )
                short_version = ""
                if version:
                    parts = str(version).split()
                    short_version = " ".join(parts[:2]) if len(parts) >= 2 else str(version)

                return ConnectionResult(
                    success=True,
                    server_version=short_version,
                    database_size=str(size_row) if size_row else "",
                    table_count=int(table_count) if table_count else 0,
                )
        except asyncpg.InvalidPasswordError:
            return ConnectionResult(
                success=False,
                message="用户名或密码错误 / Invalid username or password",
            )
        except asyncpg.InvalidCatalogNameError:
            return ConnectionResult(
                success=False,
                message=f"数据库 '{config.database}' 不存在 / "
                        f"Database '{config.database}' does not exist",
            )
        except OSError:
            return ConnectionResult(
                success=False,
                message="无法连接到服务器，请检查地址和端口 / "
                        "Cannot connect, check host and port",
            )
        except Exception:
            logger.exception("Connection test failed")
            return ConnectionResult(
                success=False, message="连接失败 / Connection failed",
            )

    async def list_databases(self, config: PostgresConfig) -> list[str]:
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            rows = await conn.fetch(
                "SELECT datname FROM pg_database "
                "WHERE datistemplate = false ORDER BY datname"
            )
            return [row["datname"] for row in rows]

    async def list_schemas(self, config: PostgresConfig) -> list[str]:
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            rows = await conn.fetch(
                "SELECT schema_name FROM information_schema.schemata "
                "WHERE schema_name NOT IN "
                "('pg_catalog', 'information_schema', 'pg_toast') "
                "ORDER BY schema_name"
            )
            return [row["schema_name"] for row in rows]

    async def list_tables(
        self, config: PostgresConfig, schema_name: str = "public",
    ) -> list[TableInfo]:
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            rows = await conn.fetch(
                "SELECT t.table_name, t.table_type, "
                "COALESCE(c.reltuples, 0)::bigint AS estimated_rows "
                "FROM information_schema.tables t "
                "LEFT JOIN pg_class c ON c.relname = t.table_name "
                "AND c.relnamespace = ("
                "  SELECT oid FROM pg_namespace WHERE nspname = $1"
                ") "
                "WHERE t.table_schema = $1 "
                "ORDER BY t.table_name",
                schema_name,
            )
            return [
                TableInfo(
                    name=row["table_name"],
                    type="view" if "VIEW" in row["table_type"] else "table",
                    estimated_rows=max(0, int(row["estimated_rows"])),
                )
                for row in rows
            ]

    async def list_columns(
        self, config: PostgresConfig, schema_name: str, table_name: str,
    ) -> list[ColumnInfo]:
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            rows = await conn.fetch(
                "SELECT column_name, data_type, udt_name, is_nullable, "
                "ordinal_position "
                "FROM information_schema.columns "
                "WHERE table_schema = $1 AND table_name = $2 "
                "ORDER BY ordinal_position",
                schema_name, table_name,
            )
            return [
                ColumnInfo(
                    name=row["column_name"],
                    data_type=row["data_type"],
                    is_nullable=row["is_nullable"] == "YES",
                    ordinal_position=row["ordinal_position"],
                )
                for row in rows
            ]

    async def get_sql_columns(
        self, config: PostgresConfig, sql: str,
    ) -> list[ColumnInfo]:
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            async with conn.transaction(readonly=True):
                stmt = await conn.prepare(
                    f"SELECT * FROM ({sql}) AS _sub LIMIT 0"
                )
                attrs = stmt.get_attributes()
                columns = []
                for attr in attrs:
                    type_oid = (
                        attr.type.oid
                        if hasattr(attr.type, "oid")
                        else attr.type
                    )
                    type_name = await self._resolve_type_name(conn, type_oid)
                    columns.append(ColumnInfo(
                        name=attr.name, data_type=type_name,
                    ))
            return columns

    async def fetch_records(
        self, config: PostgresConfig, offset: int, limit: int,
        *, schema_name: str = "public", table_name: str | None = None,
        selected_fields: list[str] | None = None, custom_sql: str | None = None,
    ) -> list[Any]:
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            if custom_sql:
                sql = custom_sql.rstrip(";")
                query = f"SELECT * FROM ({sql}) AS _sub OFFSET $1 LIMIT $2"
            else:
                if selected_fields:
                    cols = ", ".join(f'"{f}"' for f in selected_fields)
                else:
                    cols = "*"
                query = (
                    f'SELECT {cols} FROM "{schema_name}"."{table_name}" '
                    f"OFFSET $1 LIMIT $2"
                )

            async with conn.transaction(readonly=True):
                return await conn.fetch(query, offset, limit)

    async def get_primary_key_columns(
        self, config: PostgresConfig, schema_name: str, table_name: str,
    ) -> list[str]:
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            rows = await conn.fetch(
                "SELECT a.attname "
                "FROM pg_index i "
                "JOIN pg_attribute a ON a.attrelid = i.indrelid "
                "AND a.attnum = ANY(i.indkey) "
                "WHERE i.indrelid = $1::regclass AND i.indisprimary",
                f"{schema_name}.{table_name}",
            )
            return [row["attname"] for row in rows]

    async def preview_sql(
        self, config: PostgresConfig, sql: str, limit: int = 10,
    ) -> list[Any]:
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            safe_sql = sql.rstrip(";")
            query = f"SELECT * FROM ({safe_sql}) AS _sub LIMIT $1"
            async with conn.transaction(readonly=True):
                return await conn.fetch(query, limit)

    async def validate_sql(self, config: PostgresConfig, sql: str) -> bool:
        """Validate SQL using EXPLAIN to ensure it's a read-only query."""
        pm = get_pool_manager()
        async with pm.acquire(
            config.host, config.port, config.username,
            config.password, config.database,
        ) as conn:
            try:
                async with conn.transaction(readonly=True):
                    await conn.fetch(f"EXPLAIN {sql.rstrip(';')}")
                return True
            except asyncpg.PostgresError as e:
                raise ConnectorError(
                    "INVALID_SQL",
                    detail=f"SQL validation failed: {e}",
                ) from e

    def map_field_type(self, source_type: str) -> int:
        return map_pg_type(source_type)

    def can_be_primary(self, field_type: int) -> bool:
        return can_be_primary(field_type)

    def format_value(self, value: Any, field_type: int) -> Any:
        return format_value(value, field_type)

    async def close(self) -> None:
        pm = get_pool_manager()
        await pm.close_all()

    @staticmethod
    async def _resolve_type_name(
        conn: asyncpg.Connection, type_oid: int,
    ) -> str:
        row = await conn.fetchrow(
            "SELECT typname FROM pg_type WHERE oid = $1", type_oid,
        )
        if row:
            return str(row["typname"])
        return "text"
