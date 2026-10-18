use std::sync::atomic::AtomicU64;

pub(crate) static AGENT_GEN_URL: &str = "http://localhost:11434/api/generate";
pub(crate) static DEFAULT_MODEL: &str = "qwen3:8b";
pub(crate) static NEXT_RECEIVER_ID: AtomicU64 = AtomicU64::new(0);
