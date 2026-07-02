//! ADP 常量定义

/// 转发深度限制默认值
pub const FORWARD_DEPTH_LIMIT_DEFAULT: usize = 3;

/// 转发深度限制最小值
pub const FORWARD_DEPTH_LIMIT_MIN: usize = 1;

/// 转发深度限制最大值
pub const FORWARD_DEPTH_LIMIT_MAX: usize = 5;

/// 转发扇出限制默认值
pub const FORWARD_FANOUT_LIMIT_DEFAULT: usize = 10;

/// 转发扇出限制最小值
pub const FORWARD_FANOUT_LIMIT_MIN: usize = 1;

/// 转发扇出限制最大值
pub const FORWARD_FANOUT_LIMIT_MAX: usize = 20;

/// 单跳超时默认值（毫秒）
pub const FORWARD_EACH_TIMEOUT_MS_DEFAULT: u64 = 5000;

/// 总超时默认值（毫秒）
pub const FORWARD_TOTAL_TIMEOUT_MS_DEFAULT: u64 = 30000;

/// 默认返回数量限制
pub const DEFAULT_LIMIT: usize = 10;
