"""Response models conforming to Feishu Bitable connector protocol."""

from typing import Any

from pydantic import BaseModel, Field


class FieldProperty(BaseModel):
    """Field property for number/currency/progress/rating/date types.

    Args:
        formatter: Number format string (e.g., '0.00').
    """

    formatter: str | None = None


class FieldMeta(BaseModel):
    """Single field definition in table schema.

    Args:
        fieldID: Unique identifier (max 50 chars, alphanumeric + underscore).
        fieldName: Display name in Bitable (max 300 chars).
        fieldType: Bitable field type enum (1=text, 2=number, etc.).
        isPrimary: Whether this is the primary/index column.
        description: Optional field description (max 1000 chars).
        property: Optional field property for format configuration.
    """

    fieldID: str = Field(max_length=50)
    fieldName: str = Field(max_length=300)
    fieldType: int
    isPrimary: bool = False
    description: str | None = Field(default=None, max_length=1000)
    property: FieldProperty | None = None


class TableMetaData(BaseModel):
    """Table schema data returned by table_meta endpoint.

    Args:
        tableName: Name for the Bitable sync table (max 100 chars).
        fields: List of field definitions (1 to 299 fields).
    """

    tableName: str = Field(max_length=100)
    fields: list[FieldMeta] = Field(min_length=1, max_length=299)


class TableMetaResponse(BaseModel):
    """Full response for table_meta endpoint.

    Args:
        code: Status code (0 = success).
        msg: Error message (JSON string with zh/en).
        data: Table schema data.
    """

    code: int = 0
    msg: str = ""
    data: TableMetaData | None = None


class RecordData(BaseModel):
    """Single record in the records response.

    Args:
        primaryID: Unique record identifier (max 100 chars, alphanumeric + underscore).
        data: Field values keyed by fieldID.
    """

    primaryID: str = Field(max_length=100)
    data: dict[str, Any]


class RecordsData(BaseModel):
    """Records data returned by records endpoint.

    Args:
        nextPageToken: Token for the next page (empty if no more pages).
        hasMore: Whether more pages are available.
        records: List of records for current page.
    """

    nextPageToken: str = ""
    hasMore: bool = False
    records: list[RecordData] = Field(default_factory=list)


class RecordsResponse(BaseModel):
    """Full response for records endpoint.

    Args:
        code: Status code (0 = success).
        msg: Error message (JSON string with zh/en).
        data: Records data with pagination.
    """

    code: int = 0
    msg: str = ""
    data: RecordsData | None = None
