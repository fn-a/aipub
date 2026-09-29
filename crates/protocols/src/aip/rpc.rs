//! AIP JSON-RPC 传输层模型

use std::fmt;
use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value, from_str};

/// JSON-RPC 版本
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC 方法名
pub mod methods {
    /// 单次任务调用
    pub const RPC: &str = "rpc";

    /// SSE 流式调用
    pub const STREAM: &str = "stream";

    /// 注册/更新通知配置
    pub const NOTIFICATION_SET: &str = "notification/set";

    /// 删除通知配置
    pub const NOTIFICATION_DELETE: &str = "notification/delete";

    /// 查询通知配置
    pub const NOTIFICATION_GET: &str = "notification/get";

    /// 启动带通知的任务
    pub const NOTIFICATION_START: &str = "notification/start";

    /// 群组模式接入
    pub const GROUP: &str = "group";
}

/// AIP 错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AipErrorCode {
    /// JSON 解析失败
    ParseError,
    /// 请求对象非法
    InvalidRequest,
    /// 方法不存在
    MethodNotFound,
    /// 方法参数非法
    InvalidParams,
    /// 服务内部错误
    InternalError,
    /// 任务不存在
    TaskNotFound,
    /// 任务不可取消
    TaskNotCancelable,
    /// 不支持通知
    NotificationNotSupported,
    /// 不支持该操作
    UnsupportedOperation,
    /// 内容类型不兼容
    ContentTypeNotSupported,
    /// 智能体响应非法
    InvalidAgentResponse,
    /// 不支持群组通信
    GroupNotSupported,
    /// 需要认证
    AuthenticationRequired,
    /// 授权失败
    AuthorizationFailed,
    /// 访问令牌非法
    AccessTokenInvalid,
}

impl AipErrorCode {
    /// 数值错误码
    pub fn code(&self) -> i32 {
        match self {
            AipErrorCode::ParseError => -32700,
            AipErrorCode::InvalidRequest => -32600,
            AipErrorCode::MethodNotFound => -32601,
            AipErrorCode::InvalidParams => -32602,
            AipErrorCode::InternalError => -32603,
            AipErrorCode::TaskNotFound => -32001,
            AipErrorCode::TaskNotCancelable => -32002,
            AipErrorCode::NotificationNotSupported => -32003,
            AipErrorCode::UnsupportedOperation => -32004,
            AipErrorCode::ContentTypeNotSupported => -32005,
            AipErrorCode::InvalidAgentResponse => -32006,
            AipErrorCode::GroupNotSupported => -32007,
            AipErrorCode::AuthenticationRequired => -32008,
            AipErrorCode::AuthorizationFailed => -32009,
            AipErrorCode::AccessTokenInvalid => -32010,
        }
    }

    /// 默认错误消息
    ///
    /// 注意：标准 JSON-RPC 错误消息（`-32700`~`-32603`）在 ACPs 规范中
    /// 并非 JSON-RPC 官方措辞，此处与规范保持一致。
    pub fn message(&self) -> &'static str {
        match self {
            AipErrorCode::ParseError => "Invalid JSON payload",
            AipErrorCode::InvalidRequest => "Invalid JSON-RPC Request",
            AipErrorCode::MethodNotFound => "Method not found",
            AipErrorCode::InvalidParams => "Invalid method parameters",
            AipErrorCode::InternalError => "Internal server error",
            AipErrorCode::TaskNotFound => "Task not found",
            AipErrorCode::TaskNotCancelable => "Task cannot be canceled",
            AipErrorCode::NotificationNotSupported => "Notification is not supported",
            AipErrorCode::UnsupportedOperation => "This operation is not supported",
            AipErrorCode::ContentTypeNotSupported => "Incompatible content types",
            AipErrorCode::InvalidAgentResponse => "Invalid agent response type",
            AipErrorCode::GroupNotSupported => "Group communication is not supported",
            AipErrorCode::AuthenticationRequired => "Authentication required",
            AipErrorCode::AuthorizationFailed => "Authorization failed",
            AipErrorCode::AccessTokenInvalid => "Invalid access token",
        }
    }

    /// 由数值错误码还原
    pub fn from_code(code: i32) -> Option<Self> {
        const ALL: [AipErrorCode; 15] = [
            AipErrorCode::ParseError,
            AipErrorCode::InvalidRequest,
            AipErrorCode::MethodNotFound,
            AipErrorCode::InvalidParams,
            AipErrorCode::InternalError,
            AipErrorCode::TaskNotFound,
            AipErrorCode::TaskNotCancelable,
            AipErrorCode::NotificationNotSupported,
            AipErrorCode::UnsupportedOperation,
            AipErrorCode::ContentTypeNotSupported,
            AipErrorCode::InvalidAgentResponse,
            AipErrorCode::GroupNotSupported,
            AipErrorCode::AuthenticationRequired,
            AipErrorCode::AuthorizationFailed,
            AipErrorCode::AccessTokenInvalid,
        ];
        ALL.into_iter().find(|c| c.code() == code)
    }
}

/// JSON-RPC 错误对象
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    /// 错误码
    pub code: i32,

    /// 错误消息
    pub message: String,

    /// 附加数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    /// 由错误码构造
    pub fn new(code: AipErrorCode) -> Self {
        Self {
            code: code.code(),
            message: code.message().to_string(),
            data: None,
        }
    }

    /// 由错误码构造并覆盖消息
    pub fn with_message(code: AipErrorCode, message: impl Into<String>) -> Self {
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

    /// 构造 `Task not found` 错误
    pub fn task_not_found(task_id: &str) -> Self {
        Self::new(AipErrorCode::TaskNotFound).with_data(json!({ "taskId": task_id }))
    }

    /// 构造 `Invalid method parameters` 错误
    pub fn invalid_params(detail: impl Into<String>) -> Self {
        Self::with_message(AipErrorCode::InvalidParams, "Invalid method parameters")
            .with_data(Value::String(detail.into()))
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl Error for RpcError {}

/// JSON-RPC 请求
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcRequest {
    /// 固定为 `"2.0"`
    pub jsonrpc: String,

    /// 方法名
    pub method: String,

    /// 请求 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// 方法参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl RpcRequest {
    /// 构造请求
    pub fn new(method: &str, id: Value, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            id: Some(id),
            params: None,
        }
        .with_params(params)
    }

    /// 设置参数
    pub fn with_params(mut self, params: Option<Value>) -> Self {
        self.params = params;
        self
    }

    /// 从 JSON 文本解析
    pub fn parse(text: &str) -> Result<Self, RpcError> {
        let request: RpcRequest = from_str(text)
            .map_err(|e| RpcError::with_message(AipErrorCode::ParseError, e.to_string()))?;

        if request.jsonrpc != JSONRPC_VERSION {
            return Err(RpcError::new(AipErrorCode::InvalidRequest));
        }

        Ok(request)
    }

    /// 取出 `params.command`（`rpc` / `notification/start`）
    pub fn command_param(&self) -> Option<&Value> {
        self.params.as_ref().and_then(|params| params.get("command"))
    }

    /// 取出 `params.message`（`stream` 方法）
    pub fn message_param(&self) -> Option<&Value> {
        self.params.as_ref().and_then(|params| params.get("message"))
    }
}

/// JSON-RPC 响应
///
/// 与规范一致：`result` 与 `error` 互斥，且 `id` 始终存在（无法识别请求时为 `null`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcResponse {
    /// 固定为 `"2.0"`
    pub jsonrpc: String,

    /// 请求 ID（不可识别时为 null）
    pub id: Value,

    /// 成功结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// 失败错误
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    /// 成功响应
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// 失败响应
    pub fn failure(id: Value, error: RpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.result.is_some() && self.error.is_none()
    }
}
