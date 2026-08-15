mod claude;
mod common;
mod existing;
mod tencent;
pub use common::*;
pub use claude::ClaudeAdapter;
pub use existing::{CodexAdapter, HermesAdapter, OpenClawAdapter};
pub use tencent::{MarvisAdapter, WorkBuddyAdapter};
