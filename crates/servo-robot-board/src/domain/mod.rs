//! 业务逻辑领域模块
//!
//! 纯逻辑，不依赖 RTIC。按领域组织：充电、保护、电源、IMU、温度、电池、通讯。

pub mod battery;
pub mod charge;
pub mod comm;
pub mod error_stats;
pub mod event;
pub mod imu;
pub mod power;
pub mod protection;
pub mod sys_info;
pub mod thermal;
