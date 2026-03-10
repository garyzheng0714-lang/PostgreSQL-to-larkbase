"""PostgreSQL type to Feishu Bitable field type mapping."""

# Bitable field type constants
FIELD_TEXT = 1
FIELD_NUMBER = 2
FIELD_SELECT = 3
FIELD_MULTI_SELECT = 4
FIELD_DATE = 5
FIELD_BARCODE = 6
FIELD_CHECKBOX = 7
FIELD_CURRENCY = 8
FIELD_PHONE = 9
FIELD_HYPERLINK = 10
FIELD_PROGRESS = 11
FIELD_RATING = 12
FIELD_GEOLOCATION = 13

_PG_TYPE_MAP: dict[str, int] = {
    # Text types -> 1
    "text": FIELD_TEXT,
    "varchar": FIELD_TEXT,
    "character varying": FIELD_TEXT,
    "char": FIELD_TEXT,
    "character": FIELD_TEXT,
    "bpchar": FIELD_TEXT,
    "name": FIELD_TEXT,
    "uuid": FIELD_TEXT,
    "citext": FIELD_TEXT,
    "xml": FIELD_TEXT,
    "json": FIELD_TEXT,
    "jsonb": FIELD_TEXT,
    "bytea": FIELD_TEXT,
    "inet": FIELD_TEXT,
    "cidr": FIELD_TEXT,
    "macaddr": FIELD_TEXT,
    "macaddr8": FIELD_TEXT,
    "tsvector": FIELD_TEXT,
    "tsquery": FIELD_TEXT,
    "interval": FIELD_TEXT,
    "bit": FIELD_TEXT,
    "bit varying": FIELD_TEXT,
    "varbit": FIELD_TEXT,
    "pg_lsn": FIELD_TEXT,
    "enum": FIELD_TEXT,
    "user-defined": FIELD_TEXT,
    # Numeric types -> 2
    "int2": FIELD_NUMBER,
    "smallint": FIELD_NUMBER,
    "int4": FIELD_NUMBER,
    "integer": FIELD_NUMBER,
    "int": FIELD_NUMBER,
    "int8": FIELD_NUMBER,
    "bigint": FIELD_NUMBER,
    "float4": FIELD_NUMBER,
    "real": FIELD_NUMBER,
    "float8": FIELD_NUMBER,
    "double precision": FIELD_NUMBER,
    "numeric": FIELD_NUMBER,
    "decimal": FIELD_NUMBER,
    "serial": FIELD_NUMBER,
    "smallserial": FIELD_NUMBER,
    "bigserial": FIELD_NUMBER,
    "oid": FIELD_NUMBER,
    # Boolean -> 7
    "bool": FIELD_CHECKBOX,
    "boolean": FIELD_CHECKBOX,
    # Date/Time -> 5
    "date": FIELD_DATE,
    "timestamp": FIELD_DATE,
    "timestamp without time zone": FIELD_DATE,
    "timestamptz": FIELD_DATE,
    "timestamp with time zone": FIELD_DATE,
    "time": FIELD_TEXT,
    "time without time zone": FIELD_TEXT,
    "timetz": FIELD_TEXT,
    "time with time zone": FIELD_TEXT,
    # Money -> 8
    "money": FIELD_CURRENCY,
}


def map_pg_type(pg_type: str) -> int:
    """Map a PostgreSQL data type to a Bitable field type.

    Args:
        pg_type: PostgreSQL column data type name (e.g., 'varchar', 'int4').

    Returns:
        Bitable fieldType integer. Defaults to 1 (text) for unknown types.
    """
    normalized = pg_type.lower().strip()
    base_type = normalized.split("(")[0].strip()
    if base_type.endswith("[]"):
        return FIELD_TEXT
    return _PG_TYPE_MAP.get(base_type, FIELD_TEXT)


def can_be_primary(field_type: int) -> bool:
    """Check if a Bitable field type can be used as the primary/index column.

    Only text, number, date, hyperlink, phone, barcode, and currency
    types are allowed as primary columns.

    Args:
        field_type: Bitable fieldType integer.

    Returns:
        True if the type can be a primary column.
    """
    return field_type in {
        FIELD_TEXT,
        FIELD_NUMBER,
        FIELD_DATE,
        FIELD_HYPERLINK,
        FIELD_PHONE,
        FIELD_BARCODE,
        FIELD_CURRENCY,
    }
