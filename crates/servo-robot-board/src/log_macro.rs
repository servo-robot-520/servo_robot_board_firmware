//! 日志宏
//!
//! - Debug 编译: defmt 输出到 RTT + 发送给上位机
//! - Release 编译: 仅发送给上位机，不输出 RTT
//!
//! 用法:
//! ```rust
//! log_info!("电机使能");
//! log_warn!("温度过高");
//! log_error!("I2C 通信失败");
//! log_debug!("ADC 读取完成");
//! ```

/// 内部宏: 根据编译模式决定是否输出 RTT
macro_rules! _rtt_log {
    (debug, $fmt:expr $(, $args:expr)*) => {
        #[cfg(debug_assertions)]
        defmt::debug!($fmt $(, $args)*);
    };
    (info, $fmt:expr $(, $args:expr)*) => {
        #[cfg(debug_assertions)]
        defmt::info!($fmt $(, $args)*);
    };
    (warn, $fmt:expr $(, $args:expr)*) => {
        #[cfg(debug_assertions)]
        defmt::warn!($fmt $(, $args)*);
    };
    (error, $fmt:expr $(, $args:expr)*) => {
        #[cfg(debug_assertions)]
        defmt::error!($fmt $(, $args)*);
    };
}

/// Debug 级别日志
#[macro_export]
macro_rules! log_debug {
    ($fmt:expr $(, $args:expr)*) => {
        $crate::_rtt_log!(debug, $fmt $(, $args)*);
        let _ = $crate::log_task::spawn(servo_robot_protocol::log::LogMessage {
            level: servo_robot_protocol::log::LogLevel::Debug,
            file_name: alloc::string::String::from(concat!(file!(), "\0")),
            fun_name: alloc::string::String::from(concat!(module_path!(), "\0")),
            msg: alloc::string::String::from($fmt),
        });
    };
}

/// Info 级别日志
#[macro_export]
macro_rules! log_info {
    ($fmt:expr $(, $args:expr)*) => {
        $crate::_rtt_log!(info, $fmt $(, $args)*);
        let _ = $crate::log_task::spawn(servo_robot_protocol::log::LogMessage {
            level: servo_robot_protocol::log::LogLevel::Info,
            file_name: alloc::string::String::from(concat!(file!(), "\0")),
            fun_name: alloc::string::String::from(concat!(module_path!(), "\0")),
            msg: alloc::string::String::from($fmt),
        });
    };
}

/// Warn 级别日志
#[macro_export]
macro_rules! log_warn {
    ($fmt:expr $(, $args:expr)*) => {
        $crate::_rtt_log!(warn, $fmt $(, $args)*);
        let _ = $crate::log_task::spawn(servo_robot_protocol::log::LogMessage {
            level: servo_robot_protocol::log::LogLevel::Warn,
            file_name: alloc::string::String::from(concat!(file!(), "\0")),
            fun_name: alloc::string::String::from(concat!(module_path!(), "\0")),
            msg: alloc::string::String::from($fmt),
        });
    };
}

/// Error 级别日志
#[macro_export]
macro_rules! log_error {
    ($fmt:expr $(, $args:expr)*) => {
        $crate::_rtt_log!(error, $fmt $(, $args)*);
        let _ = $crate::log_task::spawn(servo_robot_protocol::log::LogMessage {
            level: servo_robot_protocol::log::LogLevel::Error,
            file_name: alloc::string::String::from(concat!(file!(), "\0")),
            fun_name: alloc::string::String::from(concat!(module_path!(), "\0")),
            msg: alloc::string::String::from($fmt),
        });
    };
}
