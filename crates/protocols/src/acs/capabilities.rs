//! ACS 能力相关辅助函数和类型

use super::AgentCapabilitySpec;
use crate::aic::validate_aic_format;

impl AgentCapabilitySpec {
    /// 验证 ACS 的基本有效性
    ///
    /// 检查必需字段是否有效，包括：
    /// - AIC 格式是否正确
    /// - 协议版本是否支持
    /// - 必需字段是否非空
    pub fn validate(&self) -> Result<(), String> {
        // 验证 AIC
        validate_aic_format(&self.aic)?;
        
        // 验证协议版本
        if self.protocol_version.is_empty() {
            return Err("协议版本不能为空".to_string());
        }
        
        // 验证名称
        if self.name.trim().is_empty() {
            return Err("智能体名称不能为空".to_string());
        }
        
        // 验证描述
        if self.description.trim().is_empty() {
            return Err("智能体描述不能为空".to_string());
        }
        
        // 验证版本号
        if self.version.trim().is_empty() {
            return Err("智能体版本号不能为空".to_string());
        }
        
        Ok(())
    }
    
    /// 检查智能体是否支持流式响应
    pub fn supports_streaming(&self) -> bool {
        self.capabilities.streaming
    }
    
    /// 检查智能体是否支持异步通知
    pub fn supports_notification(&self) -> bool {
        self.capabilities.notification
    }
    
    /// 检查智能体是否支持指定的消息队列协议
    pub fn supports_mq_protocol(&self, protocol_name: &str) -> bool {
        self.capabilities
            .message_queue
            .iter()
            .any(|mq| mq.protocol_name().eq_ignore_ascii_case(protocol_name))
    }
    
    /// 获取指定ID的技能
    pub fn get_skill(&self, skill_id: &str) -> Option<&super::AgentSkill> {
        self.skills.iter().find(|skill| skill.id == skill_id)
    }
    
    /// 检查是否拥有指定ID的技能
    pub fn has_skill(&self, skill_id: &str) -> bool {
        self.get_skill(skill_id).is_some()
    }
    
    /// 获取所有技能标签
    pub fn get_all_tags(&self) -> Vec<&str> {
        self.skills
            .iter()
            .flat_map(|skill| skill.tags.iter())
            .map(|s| s.as_str())
            .collect()
    }
    
    /// 检查是否拥有指定标签的技能
    pub fn has_skill_with_tag(&self, tag: &str) -> bool {
        self.skills
            .iter()
            .any(|skill| skill.tags.iter().any(|t| t == tag))
    }
}
