//! ADP 错误码与错误对象
//!
//! 对齐参考实现 `acps-sdk/acps_sdk/adp/errors.py` 与 `ACPs-spec-ADP-v02.01`。
//! 错误响应体为 `{"code": <int>, "message": <str>, "data"?: <object>}`。

use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// ADP 错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdpErrorCode {
    /// 容量重定向（307）
    CapacityRedirect,
    /// 区域重定向（307）
    RegionRedirect,
    /// 重定向次数超限（400）
    MaxRedirectsExceeded,
    /// 缺少查询内容（400）
    MissingQuery,
    /// 转发深度限制非法（400）
    ForwardDepthLimitInvalid,
    /// 转发链非法（400）
    ForwardChainInvalid,
    /// 过滤条件非法（400）
    FilterInvalid,
    /// 转发扇出限制非法（400）
    ForwardFanoutLimitInvalid,
    /// 证书非法（401）
    CertificateInvalid,
    /// 调用方被限流（429）
    CallerRateLimited,
    /// 租户被限流（429）
    TenantRateLimited,
    /// 检测到转发环路（508）
    ForwardLoopDetected,
    /// 转发深度超限（508）
    ForwardDepthExceeded,
    /// 转发链被篡改（508）
    ForwardChainTampered,
    /// 转发扇出超限（508）
    ForwardFanoutExceeded,
    /// 转发签名非法（508）
    ForwardSignatureInvalid,
    /// 服务内部错误（500）
    InternalError,
}

impl AdpErrorCode {
    /// 数值错误码
    pub fn code(&self) -> i32 {
        match self {
            AdpErrorCode::CapacityRedirect => 30701,
            AdpErrorCode::RegionRedirect => 30702,
            AdpErrorCode::MaxRedirectsExceeded => 30703,
            AdpErrorCode::MissingQuery => 40001,
            AdpErrorCode::ForwardDepthLimitInvalid => 40002,
            AdpErrorCode::ForwardChainInvalid => 40003,
            AdpErrorCode::FilterInvalid => 40004,
            AdpErrorCode::ForwardFanoutLimitInvalid => 40005,
            AdpErrorCode::CertificateInvalid => 40101,
            AdpErrorCode::CallerRateLimited => 42901,
            AdpErrorCode::TenantRateLimited => 42902,
            AdpErrorCode::ForwardLoopDetected => 50801,
            AdpErrorCode::ForwardDepthExceeded => 50802,
            AdpErrorCode::ForwardChainTampered => 50803,
            AdpErrorCode::ForwardFanoutExceeded => 50804,
            AdpErrorCode::ForwardSignatureInvalid => 50805,
            AdpErrorCode::InternalError => 50001,
        }
    }

    /// 符号名（默认错误消息）
    pub fn symbol(&self) -> &'static str {
        match self {
            AdpErrorCode::CapacityRedirect => "CapacityRedirect",
            AdpErrorCode::RegionRedirect => "RegionRedirect",
            AdpErrorCode::MaxRedirectsExceeded => "MaxRedirectsExceeded",
            AdpErrorCode::MissingQuery => "MissingQuery",
            AdpErrorCode::ForwardDepthLimitInvalid => "ForwardDepthLimitInvalid",
            AdpErrorCode::ForwardChainInvalid => "ForwardChainInvalid",
            AdpErrorCode::FilterInvalid => "FilterInvalid",
            AdpErrorCode::ForwardFanoutLimitInvalid => "ForwardFanoutLimitInvalid",
            AdpErrorCode::CertificateInvalid => "CertificateInvalid",
            AdpErrorCode::CallerRateLimited => "CallerRateLimited",
            AdpErrorCode::TenantRateLimited => "TenantRateLimited",
            AdpErrorCode::ForwardLoopDetected => "ForwardLoopDetected",
            AdpErrorCode::ForwardDepthExceeded => "ForwardDepthExceeded",
            AdpErrorCode::ForwardChainTampered => "ForwardChainTampered",
            AdpErrorCode::ForwardFanoutExceeded => "ForwardFanoutExceeded",
            AdpErrorCode::ForwardSignatureInvalid => "ForwardSignatureInvalid",
            AdpErrorCode::InternalError => "InternalError",
        }
    }

    /// 对应的 HTTP 状态码
    pub fn http_status(&self) -> u16 {
        match self {
            AdpErrorCode::CapacityRedirect | AdpErrorCode::RegionRedirect => 307,
            AdpErrorCode::CertificateInvalid => 401,
            AdpErrorCode::CallerRateLimited | AdpErrorCode::TenantRateLimited => 429,
            AdpErrorCode::InternalError => 500,
            AdpErrorCode::ForwardLoopDetected
            | AdpErrorCode::ForwardDepthExceeded
            | AdpErrorCode::ForwardChainTampered
            | AdpErrorCode::ForwardFanoutExceeded
            | AdpErrorCode::ForwardSignatureInvalid => 508,
            _ => 400,
        }
    }

    /// 是否为重定向类错误
    pub fn is_redirect(&self) -> bool {
        matches!(
            self,
            AdpErrorCode::CapacityRedirect | AdpErrorCode::RegionRedirect
        )
    }

    /// 是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AdpErrorCode::CallerRateLimited | AdpErrorCode::TenantRateLimited
        )
    }

    /// 是否为客户端错误（40000~40099）
    pub fn is_client_error(&self) -> bool {
        (40000..=40099).contains(&self.code())
    }

    /// 是否为转发类错误（50800~50899）
    pub fn is_forward_error(&self) -> bool {
        (50800..=50899).contains(&self.code())
    }

    /// 由数值错误码还原
    pub fn from_code(code: i32) -> Option<Self> {
        const ALL: [AdpErrorCode; 17] = [
            AdpErrorCode::CapacityRedirect,
            AdpErrorCode::RegionRedirect,
            AdpErrorCode::MaxRedirectsExceeded,
            AdpErrorCode::MissingQuery,
            AdpErrorCode::ForwardDepthLimitInvalid,
            AdpErrorCode::ForwardChainInvalid,
            AdpErrorCode::FilterInvalid,
            AdpErrorCode::ForwardFanoutLimitInvalid,
            AdpErrorCode::CertificateInvalid,
            AdpErrorCode::CallerRateLimited,
            AdpErrorCode::TenantRateLimited,
            AdpErrorCode::ForwardLoopDetected,
            AdpErrorCode::ForwardDepthExceeded,
            AdpErrorCode::ForwardChainTampered,
            AdpErrorCode::ForwardFanoutExceeded,
            AdpErrorCode::ForwardSignatureInvalid,
            AdpErrorCode::InternalError,
        ];
        ALL.into_iter().find(|c| c.code() == code)
    }
}

/// ADP 错误对象
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdpError {
    /// 错误码
    pub code: i32,

    /// 错误消息
    pub message: String,

    /// 附加数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl AdpError {
    /// 使用默认消息（符号名）创建错误
    pub fn new(code: AdpErrorCode) -> Self {
        Self {
            code: code.code(),
            message: code.symbol().to_string(),
            data: None,
        }
    }

    /// 使用自定义消息创建错误
    pub fn with_message(code: AdpErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.code(),
            message: message.into(),
            data: None,
        }
    }

    /// 附加数据
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// 对应的 HTTP 状态码
    pub fn http_status(&self) -> u16 {
        AdpErrorCode::from_code(self.code)
            .map(|c| c.http_status())
            .unwrap_or(400)
    }

    /// 转换为错误响应体
    pub fn to_error_body(&self) -> Value {
        let mut body = json!({
            "code": self.code,
            "message": self.message,
        });
        if let Some(data) = &self.data {
            body["data"] = data.clone();
        }
        body
    }
}

impl Display for AdpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl Error for AdpError {}
