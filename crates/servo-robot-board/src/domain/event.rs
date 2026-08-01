//! 事件辅助函数
//!
//! 事件触发机制: 当 board_event 状态发生变化时，由各任务调用 `event_task::spawn()` 触发。
//! event_task 内部通过 `diff_and_update` 对比前一状态，仅在有变化时发送事件帧。

use servo_robot_protocol::event::BoardEvent;

/// 对比当前 board_event 与前一状态，返回是否有变化。
///
/// 如果有变化，更新 prev 为当前状态并返回 true；否则返回 false。
/// 调用方应在同一锁内完成读取、对比和更新，避免竞态。
pub fn diff_and_update(prev: &mut BoardEvent, current: &BoardEvent) -> bool {
    if current.charge_phase != prev.charge_phase
        || current.state_change_flags != prev.state_change_flags
        || current.protection_flags != prev.protection_flags
        || current.error_flags != prev.error_flags
    {
        *prev = current.clone();
        true
    } else {
        false
    }
}
