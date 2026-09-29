//! Discovery 匹配器测试
//!
//! 测试关键词分词、技能/智能体打分与结果排名

use aipub::discovery::*;

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use super::*;

    fn candidate(aic: &str, name: &str, skills: Value) -> Value {
        json!({
            "aic": aic,
            "active": true,
            "name": name,
            "description": "提供本地服务",
            "skills": skills
        })
    }

    fn beijing_food() -> Value {
        candidate(
            "aic-food",
            "北京美食助手",
            json!([{
                "id": "beijing-tour.food",
                "name": "北京美食推荐",
                "description": "推荐北京地道美食",
                "tags": ["美食", "北京", "推荐"]
            }]),
        )
    }

    fn beijing_hotel() -> Value {
        candidate(
            "aic-hotel",
            "北京酒店助手",
            json!([{
                "id": "beijing-tour.hotel",
                "name": "北京酒店预订",
                "description": "预订北京酒店",
                "tags": ["酒店", "北京"]
            }]),
        )
    }

    #[tokio::test]
    async fn test_match_prefers_relevant_skill() {
        let matcher = KeywordMatcher::new();
        let results = matcher
            .match_agents(
                "我需要一个北京美食推荐的智能体",
                vec![beijing_food(), beijing_hotel()],
                10,
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].aic, "aic-food");
        assert_eq!(results[0].skill_id, "beijing-tour.food");
        assert_eq!(results[0].ranking, 1);
        assert!(results[0].score > results[1].score);
        assert!(results[0].score > 0.0 && results[0].score <= 1.0);
        assert!(results[0].memo.starts_with("关键词分数:"));
    }

    #[tokio::test]
    async fn test_no_match_returns_empty() {
        let matcher = KeywordMatcher::new();
        let results = matcher
            .match_agents("量子计算编译器", vec![beijing_food()], 10)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_limit_is_applied() {
        let matcher = KeywordMatcher::new();
        let results = matcher
            .match_agents("北京", vec![beijing_food(), beijing_hotel()], 1)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_empty_query_returns_empty() {
        let matcher = KeywordMatcher::new();
        let results = matcher
            .match_agents("   ", vec![beijing_food()], 10)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_tokenize_handles_cjk_and_ascii() {
        let tokens = tokenize("北京美食推荐 food-tour");
        assert!(tokens.contains(&"美".to_string()));
        assert!(tokens.contains(&"美食".to_string()));
        assert!(tokens.contains(&"food".to_string()));
        assert!(tokens.contains(&"tour".to_string()));
        assert!(tokens.contains(&"beijing".to_string()) == false);
    }

    #[test]
    fn test_english_query_matches_skill_id() {
        let tokens = tokenize("beijing tour food");
        let candidate = beijing_food();
        let skill = candidate["skills"][0].clone();
        let score = score_skill(&skill, &tokens);
        assert!(score > 0.0);
    }
}
