//! 通讯功能模块
//!
//! 帧编解码、TX/RX 队列、USB 复合设备 (CDC + MSD)、OTA 固件更新、命令处理。

pub mod command;
pub mod init;
pub mod ota;
pub mod task;
pub mod transport;
