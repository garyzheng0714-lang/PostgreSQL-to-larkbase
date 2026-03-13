"""GET /meta.json endpoint - Plugin metadata for Feishu Bitable."""

from fastapi import APIRouter
from fastapi.responses import ORJSONResponse

from src.config import settings

router = APIRouter()


@router.get("/meta.json", response_class=ORJSONResponse)
async def get_meta() -> dict[str, object]:
    """Return plugin metadata conforming to Feishu connector protocol.

    Returns:
        Plugin metadata including frontend URL, API paths, and display settings.
    """
    return {
        "schemaVersion": 1,
        "version": "1.1.0",
        "type": "data_connector",
        "extraData": {
            "disabledPeriodicSync": False,
            "dataSourceConfigUiUri": settings.frontend_url,
            "initHeight": 520,
            "initWidth": 620,
        },
        "protocol": {
            "type": "http",
            "httpProtocol": {
                "uris": [
                    {"type": "tableMeta", "uri": "/api/table_meta"},
                    {"type": "records", "uri": "/api/records"},
                ]
            },
        },
    }
