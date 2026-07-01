"""GET /meta.json endpoint - Plugin metadata for Feishu Bitable."""

import time

from fastapi import APIRouter
from fastapi.responses import ORJSONResponse

from src.config import settings

router = APIRouter()

APP_VERSION = "1.3.0"
INIT_WIDTH = 520
INIT_HEIGHT = 520


@router.get("/meta.json", response_class=ORJSONResponse)
async def get_meta() -> dict[str, object]:
    """Return plugin metadata conforming to Feishu connector protocol.

    Returns:
        Plugin metadata including frontend URL, API paths, and display settings.
    """
    cache_bust = int(time.time())
    return {
        "schemaVersion": 1,
        "version": APP_VERSION,
        "type": "data_connector",
        "extraData": {
            "disabledPeriodicSync": False,
            "dataSourceConfigUiUri": f"{settings.frontend_url}?v={cache_bust}",
            "initHeight": INIT_HEIGHT,
            "initWidth": INIT_WIDTH,
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
