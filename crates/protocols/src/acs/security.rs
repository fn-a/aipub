//! ACS 安全方案定义

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 安全方案联合类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SecurityScheme {
    /// 双向TLS认证
    MutualTls(MutualTlsSecurityScheme),
    
    /// OpenID Connect认证
    OpenIdConnect(OpenIdConnectSecurityScheme),
    
    /// API Key认证
    ApiKey(ApiKeySecurityScheme),
    
    /// HTTP认证
    Http(HttpAuthSecurityScheme),
    
    /// OAuth 2.0认证
    OAuth2(OAuth2SecurityScheme),
}

/// 双向TLS认证安全方案
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutualTlsSecurityScheme {
    /// 安全方案类型
    pub r#type: String,
    
    /// 安全方案描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// CA挑战基础URL（兼容字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "x-caChallengeBaseUrl")]
    pub x_ca_challenge_base_url: Option<String>,
}

/// OpenID Connect认证安全方案
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenIdConnectSecurityScheme {
    /// 安全方案类型
    pub r#type: String,
    
    /// 安全方案描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// OpenID Connect发现文档URL
    #[serde(rename = "openIdConnectUrl")]
    pub open_id_connect_url: String,
}

/// API Key 安全方案
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecurityScheme {
    /// 安全方案类型
    pub r#type: String,
    
    /// 安全方案描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// API密钥参数名称
    pub name: String,
    
    /// API密钥位置
    #[serde(rename = "in")]
    pub in_location: String,
}

/// HTTP 认证安全方案
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpAuthSecurityScheme {
    /// 安全方案类型
    pub r#type: String,
    
    /// 安全方案描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// HTTP认证方案名称
    pub scheme: String,
    
    /// Bearer令牌格式提示
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "bearerFormat")]
    pub bearer_format: Option<String>,
}

/// OAuth 2.0 安全方案
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2SecurityScheme {
    /// 安全方案类型
    pub r#type: String,
    
    /// 安全方案描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// OAuth2流程配置
    pub flows: OAuth2Flows,
}

/// OAuth2 流程配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Flow {
    /// 授权端点URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "authorizationUrl")]
    pub authorization_url: Option<String>,
    
    /// 令牌端点URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "tokenUrl")]
    pub token_url: Option<String>,
    
    /// 刷新令牌端点URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "refreshUrl")]
    pub refresh_url: Option<String>,
    
    /// 可用的OAuth2作用域
    #[serde(default)]
    pub scopes: HashMap<String, String>,
}

/// OAuth2 流程集合
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Flows {
    /// 隐式授权流程
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<OAuth2Flow>,
    
    /// 密码授权流程
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<OAuth2Flow>,
    
    /// 客户端凭证授权流程
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "clientCredentials")]
    pub client_credentials: Option<OAuth2Flow>,
    
    /// 授权码流程
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "authorizationCode")]
    pub authorization_code: Option<OAuth2Flow>,
}
