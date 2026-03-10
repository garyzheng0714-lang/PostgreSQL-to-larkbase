"""Application configuration loaded from environment variables."""

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    """Global application settings.

    Args:
        secret_key: Secret key for Feishu request signature verification.
        frontend_url: URL of the frontend config UI (used in meta.json).
        backend_url: Public URL of this backend service.
        cors_origins: Allowed CORS origins.
        pg_connect_timeout: PostgreSQL connection timeout in seconds.
        pg_query_timeout: PostgreSQL query timeout in seconds.
        max_row_limit: Maximum rows to sync (50000 for enterprise).
    """

    secret_key: str = "testBase"
    frontend_url: str = "http://localhost:5173"
    backend_url: str = "http://localhost:8000"
    cors_origins: list[str] = [
        "http://localhost:5173",
        "https://feishu.cn",
        "https://www.feishu.cn",
    ]
    pg_connect_timeout: int = 5
    pg_query_timeout: int = 15
    max_row_limit: int = 50000

    model_config = {"env_file": ".env", "env_file_encoding": "utf-8"}


settings = Settings()
