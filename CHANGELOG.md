# Changelog

All notable changes to this project will be documented in this file.

## [1.3.0] - 2026-03-18

### Added
- Adapter abstraction layer (`backend/src/adapters/`) enabling multi-datasource support
- `DataSourceAdapter` Protocol with 13 methods for standardized data source access
- Connection pool manager with per-config caching and TTL-based idle cleanup
- Unified `ConnectorError` exception with global FastAPI handler for Feishu protocol errors
- `list_schemas` method on adapter interface for schema discovery
- Server info display on successful connection (PostgreSQL version, database size, table count)
- Estimated row counts shown in table selector UI
- `validate_sql` method for SQL validation via EXPLAIN

### Changed
- Migrated all router endpoints from direct `pg_service` calls to adapter registry pattern
- Replaced per-request connections with pooled connections (configurable pool size, idle timeout)
- Error handling consolidated from per-endpoint try/except to centralized `wrap_adapter_exception`
- Version bumped from 1.2.0 to 1.3.0

### Removed
- `backend/src/services/pg_service.py` — replaced by `PostgresAdapter`
- `backend/src/services/type_mapper.py` — moved to `adapters/postgres/type_mapper.py`
- `backend/src/services/field_formatter.py` — moved to `adapters/postgres/formatter.py`
