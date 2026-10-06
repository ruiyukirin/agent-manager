// Author: Kirin
mod claude;
mod common;
mod deepseek;
mod existing;
mod tencent;
pub use common::*;
pub use claude::ClaudeAdapter;
pub use deepseek::DeepSeekAdapter;
pub use existing::{CodexAdapter, HermesAdapter, OpenClawAdapter};
pub use tencent::WorkBuddyAdapter;
