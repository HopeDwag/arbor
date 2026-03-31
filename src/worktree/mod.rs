mod manager;
mod status;

pub use manager::{WorktreeInfo, WorktreeManager};
pub use status::{WorktreeStatus, format_age, check_status, ahead_behind};
