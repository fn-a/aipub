//! ADP 过滤条件

use std::slice;
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::FilterOperator;

/// 单个过滤条件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterCondition {
    /// 字段路径（使用点号分隔表示嵌套）
    pub field: String,
    
    /// 匹配运算符
    pub op: FilterOperator,
    
    /// 匹配值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

impl FilterCondition {
    /// 创建新的过滤条件
    pub fn new(field: String, op: FilterOperator, value: Option<Value>) -> Self {
        Self { field, op, value }
    }
    
    /// 创建等值条件
    pub fn eq(field: &str, value: Value) -> Self {
        Self::new(field.to_string(), FilterOperator::Eq, Some(value))
    }
    
    /// 创建包含条件
    pub fn contains(field: &str, value: &str) -> Self {
        Self::new(
            field.to_string(),
            FilterOperator::Contains,
            Some(Value::String(value.to_string())),
        )
    }
    
    /// 创建存在性条件
    pub fn exists(field: &str, exists: bool) -> Self {
        Self::new(
            field.to_string(),
            FilterOperator::Exists,
            Some(Value::Bool(exists)),
        )
    }
    
    /// 创建 IN 条件
    pub fn in_values(field: &str, values: Vec<Value>) -> Self {
        Self::new(
            field.to_string(),
            FilterOperator::In,
            Some(Value::Array(values)),
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

// ============================================================================
// 过滤条件求值
// ============================================================================

/// 判断 ACS 是否命中过滤条件
///
/// `filter` 为 `None` 时视为命中（不做任何过滤）。
pub fn matches_filter(acs: &Value, filter: Option<&DiscoveryFilter>) -> bool {
    match filter {
        Some(filter) => filter.evaluate(acs),
        None => true,
    }
}

impl DiscoveryFilter {
    /// 解析逻辑关系，缺省为 `and`
    pub fn logic_kind(&self) -> super::FilterLogic {
        self.logic
            .as_deref()
            .and_then(super::FilterLogic::from_str)
            .unwrap_or_default()
    }

    /// 过滤条件的嵌套深度（根节点记为第 1 层）
    pub fn depth(&self) -> usize {
        let child_depth = self
            .groups
            .as_ref()
            .map(|groups| groups.iter().map(|g| g.depth()).max().unwrap_or(0))
            .unwrap_or(0);
        1 + child_depth
    }

    /// 对给定的 ACS 文档求值
    ///
    /// - `and`（默认）：所有条件与子条件组都命中；
    /// - `or`：任意一个条件或子条件组命中；
    /// - `not`：对「所有子节点按 `and` 组合」的结果取反。
    ///
    /// 空节点（无条件也无子条件组）在 `and`/`or` 下恒为真。
    pub fn evaluate(&self, acs: &Value) -> bool {
        let condition_results = self
            .conditions
            .as_ref()
            .map(|conditions| {
                conditions
                    .iter()
                    .map(|condition| condition.evaluate(acs))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let group_results = self
            .groups
            .as_ref()
            .map(|groups| groups.iter().map(|group| group.evaluate(acs)).collect::<Vec<_>>())
            .unwrap_or_default();

        match self.logic_kind() {
            super::FilterLogic::And => {
                condition_results.iter().all(|r| *r) && group_results.iter().all(|r| *r)
            }
            super::FilterLogic::Or => {
                let any_condition = condition_results.iter().any(|r| *r);
                let any_group = group_results.iter().any(|r| *r);
                if condition_results.is_empty() && group_results.is_empty() {
                    true
                } else {
                    any_condition || any_group
                }
            }
            super::FilterLogic::Not => {
                !(condition_results.iter().all(|r| *r) && group_results.iter().all(|r| *r))
            }
        }
    }
}

impl FilterCondition {
    /// 对给定的 ACS 文档求值
    ///
    /// 集合类运算符（`anyOf`/`allOf`/`noneOf`）在**全部**候选值上求值，
    /// 其余运算符对每个候选值求值并取「任一命中」。
    pub fn evaluate(&self, acs: &Value) -> bool {
        use super::FilterOperator as Op;

        let targets = resolve_path(acs, &self.field);
        if targets.is_empty() {
            // 字段不存在时仅 exists(false) 命中
            return matches!(self.op, Op::Exists)
                && !self.value_as_bool().unwrap_or(true);
        }

        match self.op {
            Op::AnyOf => {
                let candidates = expect_list(&self.value);
                targets
                    .iter()
                    .any(|target| collection_any(target, &candidates))
            }
            Op::AllOf => {
                let candidates = expect_list(&self.value);
                !candidates.is_empty()
                    && candidates.iter().all(|candidate| {
                        targets.iter().any(|target| {
                            collection_any(target, slice::from_ref(candidate))
                        })
                    })
            }
            Op::NoneOf => {
                let candidates = expect_list(&self.value);
                !targets
                    .iter()
                    .any(|target| collection_any(target, &candidates))
            }
            _ => targets
                .into_iter()
                .any(|target| evaluate_operator(self.op, target, &self.value)),
        }
    }

    /// 将 value 解析为布尔值
    fn value_as_bool(&self) -> Option<bool> {
        self.value.as_ref().and_then(|v| v.as_bool())
    }
}

/// 按点号分隔的路径解析出所有候选值
///
/// 路径中遇到数组时，会下沉到每个元素继续匹配（任意元素命中即视为命中），
/// 这与参考实现「数组字段的谓词作用于每个元素」的语义一致。
pub fn resolve_path<'a>(
    root: &'a Value,
    path: &str,
) -> Vec<&'a Value> {
    let segments: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    let mut out = Vec::new();
    resolve_inner(root, &segments, &mut out);
    out
}

fn resolve_inner<'a>(
    value: &'a Value,
    segments: &[&str],
    out: &mut Vec<&'a Value>,
) {
    if segments.is_empty() {
        out.push(value);
        return;
    }

    match value {
        Value::Object(map) => {
            if let Some(child) = map.get(segments[0]) {
                resolve_inner(child, &segments[1..], out);
            }
        }
        Value::Array(items) => {
            for item in items {
                resolve_inner(item, segments, out);
            }
        }
        _ => {}
    }
}

/// 判断值是否「存在且非空」
fn is_present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(s) => !s.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => true,
    }
}

/// 字符串比较（默认大小写不敏感）
fn str_eq(left: &str, right: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        left == right
    } else {
        left.eq_ignore_ascii_case(right)
    }
}

/// 值相等比较：字符串默认大小写不敏感，数字按数值比较
fn value_eq(left: &Value, right: &Value, case_sensitive: bool) -> bool {
    match (left, right) {
        (Value::String(a), Value::String(b)) => {
            str_eq(a, b, case_sensitive)
        }
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        _ => left == right,
    }
}

/// 数值/字典序比较：返回 -1/0/1，类型不可比时返回 None
fn compare(left: &Value, right: &Value) -> Option<Ordering> {
    if let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) {
        return a.partial_cmp(&b);
    }

    match (left, right) {
        (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

/// 取字符串视图
fn as_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// 取长度（数组元素数、字符串字符数、对象键数）
fn length_of(value: &Value) -> Option<usize> {
    match value {
        Value::Array(items) => Some(items.len()),
        Value::String(s) => Some(s.chars().count()),
        Value::Object(map) => Some(map.len()),
        _ => None,
    }
}

/// 取出期望值列表（数组则展开，否则视为单元素）
fn expect_list(value: &Option<Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(items)) => items.clone(),
        Some(other) => vec![other.clone()],
        None => vec![],
    }
}

/// 数组按元素展开求值（任一元素命中即可），非数组直接求值
fn any_element<F>(target: &Value, predicate: F) -> bool
where
    F: Fn(&Value) -> bool,
{
    match target {
        Value::Array(items) => items.iter().any(predicate),
        other => predicate(other),
    }
}

/// 单值谓词求值
///
/// 量词语义：
/// - 正向运算符（`eq`/`in`/`contains`/`gt` 等）在数组字段上按元素展开，任意元素命中即为命中；
/// - 否定运算符（`ne`/`nin`/`notContains`/`noneOf`/`hasNoKey`）定义为对应正向运算符取反，
///   因此数组字段上表示「没有任何元素命中」；
/// - `size*` 直接作用于数组本身；`exists` 表示「存在且非空」。
fn evaluate_operator(
    op: super::FilterOperator,
    target: &Value,
    expected: &Option<Value>,
) -> bool {
    use super::FilterOperator as Op;

    match op {
        Op::Ne => !evaluate_operator(Op::Eq, target, expected),
        Op::NeCs => !evaluate_operator(Op::EqCs, target, expected),
        Op::Nin => !evaluate_operator(Op::In, target, expected),
        Op::NinCs => !evaluate_operator(Op::InCs, target, expected),
        Op::NotContains => !evaluate_operator(Op::Contains, target, expected),
        Op::NotContainsCs => !evaluate_operator(Op::ContainsCs, target, expected),
        Op::NoneOf => !evaluate_operator(Op::AnyOf, target, expected),
        Op::HasNoKey => !evaluate_operator(Op::HasKey, target, expected),

        Op::Exists => expected
            .as_ref()
            .and_then(|v| v.as_bool())
            .map(|want| is_present(target) == want)
            .unwrap_or_else(|| is_present(target)),

        Op::Size | Op::SizeGt | Op::SizeGte | Op::SizeLt | Op::SizeLte => {
            let Some(len) = length_of(target) else {
                return false;
            };
            let Some(bound) = expected.as_ref().and_then(|v| v.as_u64()) else {
                return false;
            };
            let bound = bound as usize;
            match op {
                Op::Size => len == bound,
                Op::SizeGt => len > bound,
                Op::SizeGte => len >= bound,
                Op::SizeLt => len < bound,
                _ => len <= bound,
            }
        }

        _ => any_element(target, |value| positive_operator(op, value, expected)),
    }
}

/// 正向运算符对单个值的求值
fn positive_operator(
    op: super::FilterOperator,
    value: &Value,
    expected: &Option<Value>,
) -> bool {
    use super::FilterOperator as Op;

    match op {
        Op::Eq => expected
            .as_ref()
            .map(|v| value_eq(value, v, false))
            .unwrap_or(false),
        Op::EqCs => expected
            .as_ref()
            .map(|v| value_eq(value, v, true))
            .unwrap_or(false),

        Op::Gt | Op::Gte | Op::Lt | Op::Lte => {
            let Some(other) = expected.as_ref() else {
                return false;
            };
            let Some(ordering) = compare(value, other) else {
                return false;
            };
            match op {
                Op::Gt => ordering.is_gt(),
                Op::Gte => ordering.is_ge(),
                Op::Lt => ordering.is_lt(),
                _ => ordering.is_le(),
            }
        }

        Op::Between => {
            let bounds = expect_list(expected);
            if bounds.len() != 2 {
                return false;
            }
            match (compare(value, &bounds[0]), compare(value, &bounds[1])) {
                (Some(lower), Some(upper)) => lower.is_ge() && upper.is_le(),
                _ => false,
            }
        }

        Op::In => expect_list(expected)
            .iter()
            .any(|candidate| value_eq(value, candidate, false)),
        Op::InCs => expect_list(expected)
            .iter()
            .any(|candidate| value_eq(value, candidate, true)),

        Op::Contains | Op::StartsWith | Op::EndsWith => {
            string_predicate(op, value, expected, false)
        }
        Op::ContainsCs | Op::StartsWithCs | Op::EndsWithCs => {
            string_predicate(op, value, expected, true)
        }

        Op::AnyOf => collection_any(value, &expect_list(expected)),
        Op::AllOf => {
            let candidates = expect_list(expected);
            !candidates.is_empty()
                && candidates
                    .iter()
                    .all(|candidate| collection_any(value, slice::from_ref(candidate)))
        }

        Op::HasKey | Op::HasAnyKey | Op::HasAllKeys => {
            let Value::Object(map) = value else {
                return false;
            };
            let keys: Vec<String> = expect_list(expected)
                .iter()
                .filter_map(as_text)
                .collect();
            if keys.is_empty() {
                return false;
            }
            match op {
                Op::HasKey => keys.iter().all(|k| map.contains_key(k)),
                Op::HasAnyKey => keys.iter().any(|k| map.contains_key(k)),
                _ => keys.iter().all(|k| map.contains_key(k)),
            }
        }

        // 否定运算符与 size 已在 evaluate_operator 中处理
        _ => false,
    }
}

/// 字符串类谓词
fn string_predicate(
    op: super::FilterOperator,
    target: &Value,
    expected: &Option<Value>,
    case_sensitive: bool,
) -> bool {
    use super::FilterOperator as Op;

    let Some(needle) = expected.as_ref().and_then(as_text) else {
        return false;
    };

    // 目标为数组时对每个元素求值（任意命中即可）
    let haystacks: Vec<String> = match target {
        Value::Array(items) => items.iter().filter_map(as_text).collect(),
        other => as_text(other).into_iter().collect(),
    };

    let normalize = |s: &str| {
        if case_sensitive {
            s.to_string()
        } else {
            s.to_lowercase()
        }
    };
    let needle_norm = normalize(&needle);

    haystacks.iter().any(|haystack| {
        let haystack_norm = normalize(haystack);
        match op {
            Op::Contains | Op::ContainsCs => haystack_norm.contains(&needle_norm),
            Op::StartsWith | Op::StartsWithCs => haystack_norm.starts_with(&needle_norm),
            _ => haystack_norm.ends_with(&needle_norm),
        }
    })
}

/// 集合运算：目标为数组时按元素匹配，否则按标量匹配
fn collection_any(target: &Value, candidates: &[Value]) -> bool {
    match target {
        Value::Array(items) => items
            .iter()
            .any(|item| candidates.iter().any(|c| value_eq(item, c, false))),
        other => candidates.iter().any(|c| value_eq(other, c, false)),
    }
}
