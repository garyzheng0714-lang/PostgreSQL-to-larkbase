"""Request models for Feishu Bitable connector protocol."""

from pydantic import BaseModel


class ConnectorRequest(BaseModel):
    """Base request body from Feishu Base server.

    Args:
        params: JSON string containing datasource config and pagination info.
        context: JSON string containing system context (base token, tenant, etc.).
    """

    params: str
    context: str = "{}"


class HelperConnectionRequest(BaseModel):
    """Request for frontend helper API: test connection.

    Args:
        host: PostgreSQL server address.
        port: PostgreSQL server port.
        username: Database username.
        password: Database password.
        database: Target database name.
    """

    host: str
    port: int = 5432
    username: str
    password: str
    database: str


class HelperTablesRequest(HelperConnectionRequest):
    """Request for frontend helper API: list tables.

    Args:
        schema_name: PostgreSQL schema name.
    """

    schema_name: str = "public"


class HelperColumnsRequest(HelperTablesRequest):
    """Request for frontend helper API: list columns.

    Args:
        table_name: Table or view name.
    """

    table_name: str


class HelperSQLPreviewRequest(HelperConnectionRequest):
    """Request for frontend helper API: preview custom SQL.

    Args:
        sql: Custom SQL query to preview.
    """

    sql: str
