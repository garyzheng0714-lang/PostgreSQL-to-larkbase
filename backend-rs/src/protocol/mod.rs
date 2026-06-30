//! 飞书连接器协议层：错误码、请求/响应类型。

pub mod error;
pub mod request;
pub mod response;

pub use error::ConnectorError;
