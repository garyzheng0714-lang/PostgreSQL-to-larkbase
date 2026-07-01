"""Tests for API endpoints (meta.json and health)."""

import pytest
from httpx import AsyncClient


@pytest.mark.asyncio
class TestMetaEndpoint:
    """Test suite for GET /meta.json endpoint."""

    async def test_meta_returns_correct_structure(
        self, client: AsyncClient
    ) -> None:
        """Verify meta.json returns all required fields."""
        resp = await client.get("/meta.json")
        assert resp.status_code == 200
        data = resp.json()

        assert data["schemaVersion"] == 1
        assert data["version"] == "1.3.0"
        assert data["type"] == "data_connector"
        assert "extraData" in data
        assert "dataSourceConfigUiUri" in data["extraData"]
        assert data["extraData"]["initWidth"] == 520
        assert data["extraData"]["initHeight"] == 520
        assert "protocol" in data

    async def test_meta_has_protocol_uris(
        self, client: AsyncClient
    ) -> None:
        """Verify meta.json contains table_meta and records URIs."""
        resp = await client.get("/meta.json")
        data = resp.json()
        uris = data["protocol"]["httpProtocol"]["uris"]

        types = {u["type"] for u in uris}
        assert "tableMeta" in types
        assert "records" in types

    async def test_meta_uris_not_empty(
        self, client: AsyncClient
    ) -> None:
        """Verify URI values are non-empty strings."""
        resp = await client.get("/meta.json")
        data = resp.json()
        for uri_entry in data["protocol"]["httpProtocol"]["uris"]:
            assert uri_entry["uri"]
            assert uri_entry["uri"].startswith("/")


@pytest.mark.asyncio
class TestHealthEndpoint:
    """Test suite for GET /health endpoint."""

    async def test_health_check(self, client: AsyncClient) -> None:
        """Verify health endpoint returns ok status."""
        resp = await client.get("/health")
        assert resp.status_code == 200
        assert resp.json()["status"] == "ok"


@pytest.mark.asyncio
class TestPaginationUtils:
    """Test suite for pagination token encoding/decoding."""

    def test_encode_decode_roundtrip(self) -> None:
        """Verify encode then decode returns original offset."""
        from src.utils.pagination import (
            decode_page_token,
            encode_page_token,
        )

        for offset in [0, 100, 1000, 49000]:
            token = encode_page_token(offset)
            assert decode_page_token(token) == offset

    def test_invalid_token_raises(self) -> None:
        """Verify invalid tokens raise ValueError."""
        from src.utils.pagination import decode_page_token

        with pytest.raises(ValueError):
            decode_page_token("not_valid_base64!!!")
