//! Discovery 匹配器
//!
//! 参考实现在 GPU 模式下使用向量检索 + 重排；此处提供确定性、无外部依赖的
//! 关键词匹配实现，用于基础流程打通与本地演示。
//!
//! 打分规则（各权重体现字段的重要程度）：
//!
//! | 命中位置 | 权重 |
//! |---|---|
//! | 技能 ID | 3.0 |
//! | 技能标签 | 2.5 |
//! | 技能名称 | 2.0 |
//! | 技能描述 | 1.5 |
//! | 智能体名称 / 描述 | 1.0 |
//!
//! 最终分数 = 命中权重之和 / (查询词数 × 3.0)，归一到 `[0, 1]`。

use std::mem;
use std::cmp::Ordering;
use async_trait::async_trait;
use serde_json::Value;
use crate::DiscoveryError;

/// 匹配器 trait
#[async_trait]
pub trait Matcher: Send + Sync {
    /// 匹配智能体
    async fn match_agents(
        &self,
        query: &str,
        candidates: Vec<Value>,
        limit: usize,
    ) -> Result<Vec<MatchResult>, DiscoveryError>;
}

/// 匹配结果
#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    /// 智能体身份码
    pub aic: String,

    /// 命中的技能 ID（为空表示仅智能体级别命中）
    pub skill_id: String,

    /// 相似度分数（`[0, 1]`）
    pub score: f64,

    /// 排名（从 1 开始，越小越靠前）
    pub ranking: usize,

    /// 备注（携带分数文本，与参考实现的 `memo` 对应）
    pub memo: String,
}

impl MatchResult {
    /// 生成分数字符串备注
    pub fn score_memo(score: f64) -> String {
        format!("关键词分数: {:.4}", score)
    }
}

/// 单个字段的匹配权重
const WEIGHT_SKILL_ID: f64 = 3.0;
const WEIGHT_SKILL_TAG: f64 = 2.5;
const WEIGHT_SKILL_NAME: f64 = 2.0;
const WEIGHT_SKILL_DESCRIPTION: f64 = 1.5;
const WEIGHT_AGENT_TEXT: f64 = 1.0;

/// 单个查询词可获得的最高权重
const MAX_WEIGHT_PER_TOKEN: f64 = WEIGHT_SKILL_ID;

/// 关键词匹配器
///
/// [`SimpleMatcher`] 的别名，命名保留便于阅读。
pub struct KeywordMatcher;

impl KeywordMatcher {
    /// 创建匹配器
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeywordMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// 简单匹配器（等价于 [`KeywordMatcher`]）
pub type SimpleMatcher = KeywordMatcher;

#[async_trait]
impl Matcher for KeywordMatcher {
    async fn match_agents(
        &self,
        query: &str,
        candidates: Vec<Value>,
        limit: usize,
    ) -> Result<Vec<MatchResult>, DiscoveryError> {
        let tokens = tokenize(query);
        if tokens.is_empty() {
            return Ok(vec![]);
        }

        let mut results: Vec<MatchResult> = Vec::new();

        for candidate in &candidates {
            let Some(aic) = candidate.get("aic").and_then(|v| v.as_str()) else {
                continue;
            };

            let mut best_skill: Option<(String, f64)> = None;

            if let Some(skills) = candidate.get("skills").and_then(|v| v.as_array()) {
                for skill in skills {
                    let skill_id = skill
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let score = score_skill(skill, &tokens);
                    if score <= 0.0 {
                        continue;
                    }
                    match &best_skill {
                        Some((_, best)) if *best >= score => {}
                        _ => best_skill = Some((skill_id, score)),
                    }
                }
            }

            let agent_score = score_agent_text(candidate, &tokens);

            // 智能体级别命中时使用空技能 ID
            let (skill_id, raw_score) = match best_skill {
                Some((skill_id, score)) if score >= agent_score => (skill_id, score),
                _ if agent_score > 0.0 => (String::new(), agent_score),
                Some((skill_id, score)) => (skill_id, score),
                None => continue,
            };

            results.push(MatchResult {
                aic: aic.to_string(),
                skill_id,
                score: raw_score,
                ranking: 0,
                memo: MatchResult::score_memo(raw_score),
            });
        }

        // 分数降序；同分按 AIC 升序保证结果稳定
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.aic.cmp(&b.aic))
        });

        if limit > 0 {
            results.truncate(limit);
        }

        for (index, result) in results.iter_mut().enumerate() {
            result.ranking = index + 1;
        }

        Ok(results)
    }
}

/// 计算技能命中分数（归一化到 `[0, 1]`）
pub fn score_skill(skill: &Value, tokens: &[String]) -> f64 {
    let mut weight = 0.0;

    let id = field_text(skill, "id");
    let name = field_text(skill, "name");
    let description = field_text(skill, "description");
    let tags: Vec<String> = skill
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(|s| s.to_lowercase())
                .collect()
        })
        .unwrap_or_default();

    for token in tokens {
        let mut token_weight: f64 = 0.0;

        if contains_ci(&id, token) {
            token_weight = token_weight.max(WEIGHT_SKILL_ID);
        }
        if tags.iter().any(|tag| contains_ci(tag, token) || contains_ci(token, tag)) {
            token_weight = token_weight.max(WEIGHT_SKILL_TAG);
        }
        if contains_ci(&name, token) {
            token_weight = token_weight.max(WEIGHT_SKILL_NAME);
        }
        if contains_ci(&description, token) {
            token_weight = token_weight.max(WEIGHT_SKILL_DESCRIPTION);
        }

        weight += token_weight;
    }

    normalize(weight, tokens.len())
}

/// 计算智能体名称/描述的命中分数（归一化到 `[0, 1]`）
pub fn score_agent_text(acs: &Value, tokens: &[String]) -> f64 {
    let name = field_text(acs, "name");
    let description = field_text(acs, "description");

    let mut weight = 0.0;
    for token in tokens {
        let mut token_weight: f64 = 0.0;
        if contains_ci(&name, token) {
            token_weight = token_weight.max(WEIGHT_AGENT_TEXT);
        }
        if contains_ci(&description, token) {
            token_weight = token_weight.max(WEIGHT_AGENT_TEXT);
        }
        weight += token_weight;
    }

    normalize(weight, tokens.len())
}

/// 归一化：命中权重 / (词数 × 单词最高权重)
fn normalize(weight: f64, token_count: usize) -> f64 {
    if token_count == 0 {
        return 0.0;
    }
    (weight / (token_count as f64 * MAX_WEIGHT_PER_TOKEN)).clamp(0.0, 1.0)
}

/// 取字段文本（小写）
fn field_text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase()
}

/// 大小写不敏感的子串包含
fn contains_ci(haystack: &str, needle: &str) -> bool {
    !needle.is_empty() && haystack.contains(needle)
}

/// 查询分词
///
/// - 连续 ASCII 字母/数字视为一个词；
/// - 中日韩文字按单字切分，并额外生成相邻二元组（提升中文召回）。
pub fn tokenize(query: &str) -> Vec<String> {
    let lowered = query.to_lowercase();
    let mut tokens: Vec<String> = Vec::new();
    let mut ascii_buffer = String::new();
    let mut cjk_buffer: Vec<char> = Vec::new();

    let flush_ascii = |buffer: &mut String, tokens: &mut Vec<String>| {
        if !buffer.is_empty() {
            tokens.push(mem::take(buffer));
        }
    };

    let flush_cjk = |buffer: &mut Vec<char>, tokens: &mut Vec<String>| {
        if buffer.is_empty() {
            return;
        }
        for ch in buffer.iter() {
            tokens.push(ch.to_string());
        }
        for window in buffer.windows(2) {
            tokens.push(window.iter().collect());
        }
        buffer.clear();
    };

    for ch in lowered.chars() {
        if is_cjk(ch) {
            flush_ascii(&mut ascii_buffer, &mut tokens);
            cjk_buffer.push(ch);
        } else if ch.is_alphanumeric() {
            flush_cjk(&mut cjk_buffer, &mut tokens);
            ascii_buffer.push(ch);
        } else {
            flush_ascii(&mut ascii_buffer, &mut tokens);
            flush_cjk(&mut cjk_buffer, &mut tokens);
        }
    }
    flush_ascii(&mut ascii_buffer, &mut tokens);
    flush_cjk(&mut cjk_buffer, &mut tokens);

    tokens.sort();
    tokens.dedup();
    tokens
}

/// 是否中日韩文字
fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x3040..=0x30FF      // 日文假名
        | 0x3400..=0x4DBF    // 扩展 A
        | 0x4E00..=0x9FFF    // 基本汉字
        | 0xF900..=0xFAFF    // 兼容汉字
        | 0xAC00..=0xD7AF    // 韩文
    )
}
