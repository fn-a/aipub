//! # ADP - Agent Discovery Protocol (智能体发现协议)
//!
//! ADP 定义了智能体发现流程，支持智能体能力查询、过滤和跨域发现。
//!
//! ## 主要结构
//!
//! - [`DiscoveryRequest`]: 发现请求
//! - [`DiscoveryResponse`]: 发现响应
//! - [`DiscoveryFilter`]: 过滤条件
//! - [`DiscoveryResult`]: 发现结果
//!
//! ## 查询类型
//!
//! ADP 支持多种查询类型：
//!
//! - `explicit`: 明确查询（有明确意图）
//! - `exploratory`: 探索性查询（无明确目标）
//! - `trending`: 热门查询
//! - `filtered`: 过滤查询
//!
//! ## 转发机制
//!
//! ADP 支持跨域发现服务转发，通过转发链实现跨域智能体发现。

mod models;
mod filter;
mod constants;

pub use models::*;
pub use filter::*;
pub use constants::*;

use serde::{Deserialize, Serialize};

/// 查询类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryType {
    /// 明确查询（有明确意图并按 filter 过滤）
    Explicit,
    
    /// 探索性查询（用户没有明确目标）
    Exploratory,
    
    /// 热门查询（返回当前流行的智能体）
    Trending,
    
    /// 过滤查询（只用 filter 过滤，query 被忽略）
    Filtered,
}

impl Default for QueryType {
    fn default() -> Self {
        QueryType::Explicit
    }
}

/// 过滤运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterOperator {
    // 通用：等值与存在性
    #[serde(rename = "eq")]
    Eq,
    #[serde(rename = "ne")]
    Ne,
    #[serde(rename = "exists")]
    Exists,
    
    // 比较：数值、日期、字符串字典序
    #[serde(rename = "gt")]
    Gt,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "lte")]
    Lte,
    #[serde(rename = "between")]
    Between,
    
    // 集合：值列表匹配
    #[serde(rename = "in")]
    In,
    #[serde(rename = "nin")]
    Nin,
    
    // 字符串：模式匹配（默认大小写不敏感）
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "notContains")]
    NotContains,
    #[serde(rename = "startsWith")]
    StartsWith,
    #[serde(rename = "endsWith")]
    EndsWith,
    
    // 字符串：大小写敏感变体
    #[serde(rename = "eqCs")]
    EqCs,
    #[serde(rename = "neCs")]
    NeCs,
    #[serde(rename = "inCs")]
    InCs,
    #[serde(rename = "ninCs")]
    NinCs,
    #[serde(rename = "containsCs")]
    ContainsCs,
    #[serde(rename = "notContainsCs")]
    NotContainsCs,
    #[serde(rename = "startsWithCs")]
    StartsWithCs,
    #[serde(rename = "endsWithCs")]
    EndsWithCs,
    
    // 数组：集合运算
    #[serde(rename = "anyOf")]
    AnyOf,
    #[serde(rename = "allOf")]
    AllOf,
    #[serde(rename = "noneOf")]
    NoneOf,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "sizeGt")]
    SizeGt,
    #[serde(rename = "sizeGte")]
    SizeGte,
    #[serde(rename = "sizeLt")]
    SizeLt,
    #[serde(rename = "sizeLte")]
    SizeLte,
    
    // Map/对象：键检测
    #[serde(rename = "hasKey")]
    HasKey,
    #[serde(rename = "hasNoKey")]
    HasNoKey,
    #[serde(rename = "hasAnyKey")]
    HasAnyKey,
    #[serde(rename = "hasAllKeys")]
    HasAllKeys,
}
