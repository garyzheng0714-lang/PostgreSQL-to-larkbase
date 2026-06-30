/** 数据源类型注册表 —— 加一种数据库 = 这里加一行，UI 不用重设计。 */
export interface SourceType {
  id: string;
  name: string;
  /** 类型芯片里的单色字形（不依赖品牌彩色 logo） */
  badge: string;
  active: boolean;
}

export const SOURCE_TYPES: SourceType[] = [
  { id: "postgres", name: "PostgreSQL", badge: "Pg", active: true },
  { id: "mysql", name: "MySQL", badge: "My", active: false },
  { id: "mongodb", name: "MongoDB", badge: "Mo", active: false },
  { id: "sqlserver", name: "SQL Server", badge: "Ss", active: false },
  { id: "sqlite", name: "SQLite", badge: "Sl", active: false },
  { id: "clickhouse", name: "ClickHouse", badge: "Ch", active: false },
];

export const DEFAULT_SOURCE = SOURCE_TYPES[0];

export function getSource(id: string): SourceType {
  return SOURCE_TYPES.find((s) => s.id === id) ?? DEFAULT_SOURCE;
}
