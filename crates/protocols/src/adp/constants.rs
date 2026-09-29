//! ADP 常量定义
//!
//! 取值对齐参考实现（`acps-sdk/acps_sdk/adp/models.py` 与 `ACPs-spec-ADP-v02.01`）。

/// 发现服务端点路径
pub const DISCOVERY_ENDPOINT_PATH: &str = "/discover";

/// 转发深度限制默认值
pub const FORWARD_DEPTH_LIMIT_DEFAULT: usize = 3;

/// 转发深度限制最小值
pub const FORWARD_DEPTH_LIMIT_MIN: usize = 1;

/// 转发深度限制最大值
pub const FORWARD_DEPTH_LIMIT_MAX: usize = 5;

/// 转发扇出限制默认值
pub const FORWARD_FANOUT_LIMIT_DEFAULT: usize = 1;

/// 转发扇出限制最小值
pub const FORWARD_FANOUT_LIMIT_MIN: usize = 1;

/// 转发扇出限制最大值
pub const FORWARD_FANOUT_LIMIT_MAX: usize = 5;

/// 单跳超时默认值（毫秒）
pub const FORWARD_EACH_TIMEOUT_MS_DEFAULT: u64 = 10000;

/// 总超时默认值（毫秒）
pub const FORWARD_TOTAL_TIMEOUT_MS_DEFAULT: u64 = 60000;

/// 默认返回数量限制
pub const DEFAULT_LIMIT: usize = 10;

/// 过滤条件最大嵌套深度（建议上限）
pub const FILTER_MAX_NESTING_DEPTH: usize = 3;

/// 发现上下文最大负载字节数（建议值）
pub const CONTEXT_MAX_PAYLOAD_BYTES: usize = 2048;

/// 最大重定向跳数
pub const MAX_REDIRECT_HOPS: usize = 5;
