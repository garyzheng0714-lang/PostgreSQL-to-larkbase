export interface ConnectionInfo {
  host: string;
  port: number;
  username: string;
  password: string;
  database: string;
}

export interface TableInfo {
  name: string;
  type: "table" | "view";
}

export interface ColumnInfo {
  name: string;
  data_type: string;
  udt_name: string;
  is_nullable: boolean;
  ordinal_position: number;
  bitable_type: number;
}

export interface NumberFormat {
  precision: number;
}

export interface DatasourceConfig {
  host: string;
  port: number;
  username: string;
  password: string;
  database: string;
  mode: "table" | "sql";
  schema_name: string;
  table_name: string | null;
  selected_fields: string[] | null;
  custom_sql: string | null;
  field_renames: Record<string, string> | null;
  number_formats: Record<string, NumberFormat> | null;
  auto_sync: boolean;
}

export interface SQLPreviewResult {
  columns: { name: string; data_type: string }[];
  rows: Record<string, unknown>[];
}

export type StepKey = "connection" | "table" | "fields" | "confirm";

export const BITABLE_TYPE_LABELS: Record<number, string> = {
  1: "文本",
  2: "数字",
  3: "单选",
  4: "多选",
  5: "日期",
  6: "条码",
  7: "复选框",
  8: "货币",
  9: "电话",
  10: "超链接",
  11: "进度",
  12: "评分",
  13: "地理位置",
};
