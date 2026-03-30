"""Connection pool manager with per-config caching and TTL cleanup.

Architecture:
  request → pool_key = hash(host, port, user, db)
              │
    ┌─────────┼──────────┐
    │ HIT     │ MISS     │ CLEANUP (every 60s)
    ▼         ▼          ▼
  reuse     create     close idle > TTL
  pool      new pool   pools
"""

from __future__ import annotations

import asyncio
import logging
import time
from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager
from typing import Any

import asyncpg

logger = logging.getLogger(__name__)


def _make_pool_key(
    host: str, port: int, user: str, database: str,
    ssl_mode: str = "disable",
) -> str:
    return f"{host}:{port}:{user}:{database}:{ssl_mode}"


class _PoolEntry:
    __slots__ = ("pool", "last_used", "key")

    def __init__(self, pool: asyncpg.Pool, key: str) -> None:
        self.pool = pool
        self.last_used = time.monotonic()
        self.key = key

    def touch(self) -> None:
        self.last_used = time.monotonic()


class ConnectionPoolManager:
    """Manages a cache of asyncpg connection pools keyed by connection params."""

    def __init__(
        self,
        max_pool_size: int = 5,
        idle_timeout: float = 300.0,
        max_pools: int = 20,
        connect_timeout: float = 5.0,
        command_timeout: float = 15.0,
    ) -> None:
        self._pools: dict[str, _PoolEntry] = {}
        self._max_pool_size = max_pool_size
        self._idle_timeout = idle_timeout
        self._max_pools = max_pools
        self._connect_timeout = connect_timeout
        self._command_timeout = command_timeout
        self._lock = asyncio.Lock()
        self._cleanup_task: asyncio.Task[None] | None = None

    async def _get_or_create_pool(
        self,
        host: str,
        port: int,
        user: str,
        password: str,
        database: str,
        ssl: Any = None,
        ssl_mode: str = "disable",
        connect_timeout: float | None = None,
        command_timeout: float | None = None,
    ) -> asyncpg.Pool:
        key = _make_pool_key(host, port, user, database, ssl_mode)

        async with self._lock:
            entry = self._pools.get(key)
            if entry is not None:
                entry.touch()
                return entry.pool

            if len(self._pools) >= self._max_pools:
                await self._evict_oldest()

            pool = await asyncpg.create_pool(
                host=host,
                port=port,
                user=user,
                password=password,
                database=database,
                ssl=ssl,
                min_size=1,
                max_size=self._max_pool_size,
                timeout=connect_timeout or self._connect_timeout,
                command_timeout=command_timeout or self._command_timeout,
            )
            self._pools[key] = _PoolEntry(pool, key)
            logger.info(
                "Created connection pool: %s:%d/%s (total: %d)",
                host, port, database, len(self._pools),
            )
            return pool

    async def _evict_oldest(self) -> None:
        if not self._pools:
            return
        oldest_key = min(self._pools, key=lambda k: self._pools[k].last_used)
        entry = self._pools.pop(oldest_key)
        await entry.pool.close()
        logger.info("Evicted oldest pool: %s", oldest_key)

    @asynccontextmanager
    async def acquire(
        self,
        host: str,
        port: int,
        user: str,
        password: str,
        database: str,
        ssl: Any = None,
        ssl_mode: str = "disable",
        connect_timeout: float | None = None,
        command_timeout: float | None = None,
    ) -> AsyncGenerator[asyncpg.Connection, None]:
        pool = await self._get_or_create_pool(
            host, port, user, password, database,
            ssl=ssl, ssl_mode=ssl_mode,
            connect_timeout=connect_timeout, command_timeout=command_timeout,
        )
        async with pool.acquire() as conn:
            yield conn

    async def cleanup_idle(self) -> None:
        now = time.monotonic()
        to_remove: list[str] = []

        async with self._lock:
            for key, entry in self._pools.items():
                if now - entry.last_used > self._idle_timeout:
                    to_remove.append(key)

            for key in to_remove:
                entry = self._pools.pop(key)
                await entry.pool.close()
                logger.info("Cleaned up idle pool: %s", key)

    async def start_cleanup_loop(self, interval: float = 60.0) -> None:
        async def _loop() -> None:
            while True:
                await asyncio.sleep(interval)
                try:
                    await self.cleanup_idle()
                except Exception:
                    logger.exception("Pool cleanup error")

        self._cleanup_task = asyncio.create_task(_loop())

    async def close_all(self) -> None:
        if self._cleanup_task is not None:
            self._cleanup_task.cancel()
            self._cleanup_task = None

        async with self._lock:
            for entry in self._pools.values():
                await entry.pool.close()
            count = len(self._pools)
            self._pools.clear()
            logger.info("Closed all %d connection pools", count)


pool_manager: ConnectionPoolManager | None = None


def get_pool_manager() -> ConnectionPoolManager:
    global pool_manager
    if pool_manager is None:
        from src.config import settings
        pool_manager = ConnectionPoolManager(
            max_pool_size=settings.pool_max_size,
            idle_timeout=settings.pool_idle_timeout,
            max_pools=settings.pool_max_pools,
            connect_timeout=settings.pg_connect_timeout,
            command_timeout=settings.pg_query_timeout,
        )
    return pool_manager
