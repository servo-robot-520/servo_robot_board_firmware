//! 事件辅助函数
//!
//! 各功能任务更新共享的 `board_event` 快照后调用 `event_flush_task::spawn()`。
//! 该低优先级任务集中做 diff、去重和发送；它是最新状态的通知，不是可靠的状态迁移队列。

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
