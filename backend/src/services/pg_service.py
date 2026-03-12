"""PostgreSQL connection management and query operations."""

import logging
from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
from typing import Any

import asyncpg

from src.config import settings
from src.models.datasource import DatasourceConfig

logger = logging.getLogger(__name__)


@asynccontextmanager
async def get_connection(
    config: DatasourceConfig,
) -> AsyncGenerator[asyncpg.Connection, None]:
    """Create and manage a PostgreSQL connection.

    Args:
        config: Datasource configuration with connection details.

    Yields:
        asyncpg.Connection instance.

    Raises:
        asyncpg.PostgresError: On connection or query failures.
    """
    conn = await asyncpg.connect(
        host=config.host,
        port=config.port,
        user=config.username,
        password=config.password,
        database=config.database,
        timeout=settings.pg_connect_timeout,
        command_timeout=settings.pg_query_timeout,
    )
    try:
        yield conn
    finally:
        await conn.close()


async def test_connection(config: DatasourceConfig) -> bool:
    """Test if a PostgreSQL connection can be established.

    Args:
        config: Datasource configuration with connection details.

    Returns:
        True if connection succeeds.

    Raises:
        asyncpg.PostgresError: If connection fails.
    """
    async with get_connection(config) as conn:
        await conn.fetchval("SELECT 1")
        return True


async def list_databases(config: DatasourceConfig) -> list[str]:
    """List all non-template databases accessible by the user.

    Args:
        config: Datasource configuration with connection details.

    Returns:
        Sorted list of database names.
    """
    async with get_connection(config) as conn:
        rows = await conn.fetch(
            "SELECT datname FROM pg_database "
            "WHERE datistemplate = false ORDER BY datname"
        )
        return [row["datname"] for row in rows]


async def list_schemas(config: DatasourceConfig) -> list[str]:
    """List all user-accessible schemas in the connected database.

    Args:
        config: Datasource configuration with connection details.

    Returns:
        Sorted list of schema names (excluding system schemas).
    """
    async with get_connection(config) as conn:
        rows = await conn.fetch(
            "SELECT schema_name FROM information_schema.schemata "
            "WHERE schema_name NOT IN ('pg_catalog', 'information_schema', "
            "'pg_toast') ORDER BY schema_name"
        )
        return [row["schema_name"] for row in rows]


async def list_tables(
    config: DatasourceConfig,
    schema_name: str = "public",
) -> list[dict[str, str]]:
    """List all tables and views in a schema.

    Args:
        config: Datasource configuration with connection details.
        schema_name: PostgreSQL schema name.

    Returns:
        List of dicts with 'name' and 'type' ('table' or 'view') keys.
    """
    async with get_connection(config) as conn:
        rows = await conn.fetch(
            "SELECT table_name, table_type FROM information_schema.tables "
            "WHERE table_schema = $1 ORDER BY table_name",
            schema_name,
        )
        return [
            {
                "name": row["table_name"],
                "type": "view" if "VIEW" in row["table_type"] else "table",
            }
            for row in rows
        ]


async def list_columns(
    config: DatasourceConfig,
    schema_name: str,
    table_name: str,
) -> list[dict[str, Any]]:
    """List all columns for a specific table or view.

    Args:
        config: Datasource configuration with connection details.
        schema_name: PostgreSQL schema name.
        table_name: Table or view name.

    Returns:
        List of column info dicts with keys: name, data_type, udt_name,
        is_nullable, ordinal_position.
    """
    async with get_connection(config) as conn:
        rows = await conn.fetch(
            "SELECT column_name, data_type, udt_name, is_nullable, "
            "ordinal_position "
            "FROM information_schema.columns "
            "WHERE table_schema = $1 AND table_name = $2 "
            "ORDER BY ordinal_position",
            schema_name,
            table_name,
        )
        return [
            {
                "name": row["column_name"],
                "data_type": row["data_type"],
                "udt_name": row["udt_name"],
                "is_nullable": row["is_nullable"] == "YES",
                "ordinal_position": row["ordinal_position"],
            }
            for row in rows
        ]


async def get_primary_key_columns(
    config: DatasourceConfig,
    schema_name: str,
    table_name: str,
) -> list[str]:
    """Get primary key column names for a table.

    Args:
        config: Datasource configuration with connection details.
        schema_name: PostgreSQL schema name.
        table_name: Table name.

    Returns:
        List of primary key column names (empty if no PK).
    """
    async with get_connection(config) as conn:
        rows = await conn.fetch(
            "SELECT a.attname "
            "FROM pg_index i "
            "JOIN pg_attribute a ON a.attrelid = i.indrelid "
            "AND a.attnum = ANY(i.indkey) "
            "WHERE i.indrelid = $1::regclass AND i.indisprimary",
            f"{schema_name}.{table_name}",
        )
        return [row["attname"] for row in rows]


async def get_sql_columns(
    config: DatasourceConfig,
    sql: str,
) -> list[dict[str, str]]:
    """Infer column metadata from a custom SQL query by executing LIMIT 0.

    Args:
        config: Datasource configuration with connection details.
        sql: Custom SQL query.

    Returns:
        List of column info dicts with keys: name, data_type.
    """
    async with get_connection(config) as conn:
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
                type_name = await _resolve_type_name(conn, type_oid)
                columns.append({"name": attr.name, "data_type": type_name})
        return columns


async def _resolve_type_name(conn: asyncpg.Connection, type_oid: int) -> str:
    """Resolve a PostgreSQL type OID to its name.

    Args:
        conn: Active database connection.
        type_oid: PostgreSQL type OID.

    Returns:
        Type name string.
    """
    row = await conn.fetchrow(
        "SELECT typname FROM pg_type WHERE oid = $1", type_oid
    )
    if row:
        return str(row["typname"])
    return "text"


async def fetch_records(
    config: DatasourceConfig,
    offset: int,
    limit: int,
) -> list[Any]:
    """Fetch paginated records from the configured data source.

    Args:
        config: Datasource configuration (table mode or SQL mode).
        offset: Row offset for pagination.
        limit: Maximum number of rows to return.

    Returns:
        List of asyncpg.Record objects.
    """
    async with get_connection(config) as conn:
        if config.mode == "sql" and config.custom_sql:
            sql = config.custom_sql.rstrip(";")
            query = f"SELECT * FROM ({sql}) AS _sub OFFSET $1 LIMIT $2"
        else:
            schema = config.schema_name or "public"
            table = config.table_name
            if config.selected_fields:
                cols = ", ".join(
                    f'"{f}"' for f in config.selected_fields
                )
            else:
                cols = "*"
            query = (
                f'SELECT {cols} FROM "{schema}"."{table}" '
                f"OFFSET $1 LIMIT $2"
            )

        async with conn.transaction(readonly=True):
            result: list[Any] = await conn.fetch(query, offset, limit)
        return result


async def preview_sql(
    config: DatasourceConfig,
    sql: str,
    limit: int = 10,
) -> list[Any]:
    """Preview results of a custom SQL query (limited rows).

    Args:
        config: Datasource configuration with connection details.
        sql: Custom SQL query to preview.
        limit: Maximum preview rows (default 10).

    Returns:
        List of asyncpg.Record objects.
    """
    async with get_connection(config) as conn:
        safe_sql = sql.rstrip(";")
        query = f"SELECT * FROM ({safe_sql}) AS _sub LIMIT $1"
        async with conn.transaction(readonly=True):
            result: list[Any] = await conn.fetch(query, limit)
        return result
