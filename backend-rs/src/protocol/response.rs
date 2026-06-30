//! 飞书连接器协议响应类型（对应 Python `response.py`）。
//!
//! 字段 casing 锁死，以已上线 Python 为准：`fieldID`/`fieldName`/`fieldType`/
//! `isPrimary`/`primaryID`/`nextPageToken`/`hasMore`。官方 Node demo 的
//! `fieldId`/`primaryId`（小写 d）是错的，不采用。

use serde::Serialize;
use serde_json::Value;

/// 字段 property（数字/货币/进度/评分/日期格式）。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FieldProperty {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatter: Option<String>,
}

/// 表结构中的单个字段定义。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FieldMeta {
    #[serde(rename = "fieldID")]
    pub field_id: String,
    #[serde(rename = "fieldName")]
    pub field_name: String,
    #[serde(rename = "fieldType")]
    pub field_type: i32,
    #[serde(rename = "isPrimary")]
    pub is_primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<FieldProperty>,
}

/// table_meta 端点返回的表结构数据。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TableMetaData {
    #[serde(rename = "tableName")]
    pub table_name: String,
    pub fields: Vec<FieldMeta>,
}

/// records 端点返回的单条记录。`data` 的 key 为 fieldID。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RecordData {
    #[serde(rename = "primaryID")]
    pub primary_id: String,
    pub data: std::collections::BTreeMap<String, Value>,
}

/// records 端点返回的分页数据。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RecordsData {
    #[serde(rename = "nextPageToken")]
    pub next_page_token: String,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    pub records: Vec<RecordData>,
}

/// 构造成功响应体 `{code:0, msg:"", data}`。
pub fn ok_body<T: Serialize>(data: T) -> Value {
    serde_json::json!({ "code": 0, "msg": "", "data": data })
}
