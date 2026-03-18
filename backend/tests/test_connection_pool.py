"""Tests for the connection pool manager."""

from src.adapters.postgres.pool import ConnectionPoolManager, _make_pool_key


class TestPoolKey:

    def test_same_params_same_key(self) -> None:
        k1 = _make_pool_key("localhost", 5432, "user", "db")
        k2 = _make_pool_key("localhost", 5432, "user", "db")
        assert k1 == k2

    def test_different_host_different_key(self) -> None:
        k1 = _make_pool_key("host1", 5432, "user", "db")
        k2 = _make_pool_key("host2", 5432, "user", "db")
        assert k1 != k2

    def test_different_port_different_key(self) -> None:
        k1 = _make_pool_key("host", 5432, "user", "db")
        k2 = _make_pool_key("host", 5433, "user", "db")
        assert k1 != k2

    def test_different_db_different_key(self) -> None:
        k1 = _make_pool_key("host", 5432, "user", "db1")
        k2 = _make_pool_key("host", 5432, "user", "db2")
        assert k1 != k2


class TestConnectionPoolManager:

    def test_init_defaults(self) -> None:
        pm = ConnectionPoolManager()
        assert pm._max_pool_size == 5
        assert pm._idle_timeout == 300.0
        assert pm._max_pools == 20

    def test_init_custom(self) -> None:
        pm = ConnectionPoolManager(
            max_pool_size=10,
            idle_timeout=60.0,
            max_pools=5,
        )
        assert pm._max_pool_size == 10
        assert pm._idle_timeout == 60.0
        assert pm._max_pools == 5

    def test_empty_pools(self) -> None:
        pm = ConnectionPoolManager()
        assert len(pm._pools) == 0
