//! ADP 过滤条件

use super::FilterOperator;
use serde::{Deserialize, Serialize};

/// 单个过滤条件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterCondition {
    /// 字段路径（使用点号分隔表示嵌套）
    pub field: String,
    
    /// 匹配运算符
    pub op: FilterOperator,
    
    /// 匹配值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

impl FilterCondition {
    /// 创建新的过滤条件
    pub fn new(field: String, op: FilterOperator, value: Option<serde_json::Value>) -> Self {
        Self { field, op, value }
    }
    
    /// 创建等值条件
    pub fn eq(field: &str, value: serde_json::Value) -> Self {
        Self::new(field.to_string(), FilterOperator::Eq, Some(value))
    }
    
    /// 创建包含条件
    pub fn contains(field: &str, value: &str) -> Self {
        Self::new(
            field.to_string(),
            FilterOperator::Contains,
            Some(serde_json::Value::String(value.to_string())),
        )
    }
    
    /// 创建存在性条件
    pub fn exists(field: &str, exists: bool) -> Self {
        Self::new(
            field.to_string(),
            FilterOperator::Exists,
            Some(serde_json::Value::Bool(exists)),
        )
    }
    
    /// 创建 IN 条件
    pub fn in_values(field: &str, values: Vec<serde_json::Value>) -> Self {
        Self::new(
            field.to_string(),
            FilterOperator::In,
            Some(serde_json::Value::Array(values)),
        )
    }
}

/// 过滤条件集合
///
/// 支持逻辑组合（AND/OR/NOT），可对 ACS 中所有可查询字段进行匹配
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryFilter {
    /// 过滤条件列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<FilterCondition>>,
    
    /// 嵌套的子条件组
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<DiscoveryFilter>>,
    
    /// 逻辑关系（and / or / not）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logic: Option<String>,
}

impl DiscoveryFilter {
    /// 创建新的过滤条件集合
    pub fn new() -> Self {
        Self {
            conditions: None,
            groups: None,
            logic: None,
        }
    }
    
    /// 创建 AND 过滤器
    pub fn and(conditions: Vec<FilterCondition>) -> Self {
        Self {
            conditions: Some(conditions),
            groups: None,
            logic: Some("and".to_string()),
        }
    }
    
    /// 创建 OR 过滤器
    pub fn or(conditions: Vec<FilterCondition>) -> Self {
        Self {
            conditions: Some(conditions),
            groups: None,
            logic: Some("or".to_string()),
        }
    }
    
    /// 创建 NOT 过滤器
    pub fn not(condition: FilterCondition) -> Self {
        Self {
            conditions: Some(vec![condition]),
            groups: None,
            logic: Some("not".to_string()),
        }
    }
    
    /// 添加条件
    pub fn add_condition(mut self, condition: FilterCondition) -> Self {
        self.conditions
            .get_or_insert_with(Vec::new)
            .push(condition);
        self
    }
    
    /// 设置逻辑关系
    pub fn with_logic(mut self, logic: &str) -> Self {
        self.logic = Some(logic.to_string());
        self
    }
}

impl Default for DiscoveryFilter {
    fn default() -> Self {
        Self::new()
    }
}
