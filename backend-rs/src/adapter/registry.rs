//! 适配器注册表（对应 Python `registry.py`）。

use std::collections::HashMap;
use std::sync::Arc;

use super::DataSourceAdapter;

/// 按数据源类型查找适配器。第一个注册的为默认。
#[derive(Default)]
pub struct Registry {
    adapters: HashMap<String, Arc<dyn DataSourceAdapter>>,
    default: Option<String>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, adapter: Arc<dyn DataSourceAdapter>) {
        let t = adapter.source_type().to_string();
        if self.default.is_none() {
            self.default = Some(t.clone());
        }
        self.adapters.insert(t, adapter);
    }

    pub fn get(&self, source_type: &str) -> Option<Arc<dyn DataSourceAdapter>> {
        self.adapters.get(source_type).cloned()
    }

    pub fn get_default(&self) -> Option<Arc<dyn DataSourceAdapter>> {
        self.default.as_ref().and_then(|t| self.get(t))
    }
}
