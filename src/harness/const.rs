use crate::prelude::*;

pub const TODO: &'static str = r#"
	- Question prompt
	- Add progressive disclosure
"#;

pub const AH_GOAL: &str = include_str!("input/user.goal.md");

pub const AH_LOG_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/log");
pub const AH_LOG_FILE_NAME: &str = "events.jsonl";

pub const AH_PROMPT_INTENT: &str = include_str!("input/prompt.writer.intent.md");
pub const AH_PROMPT_SPEC: &str = include_str!("input/prompt.writer.spec.md");
pub const AH_PROMPT_PLAN: &str = include_str!("input/prompt.writer.plan.md");

pub const AH_TEMPLATE_INTENT: &str = include_str!("input/intent.md");
pub const AH_TEMPLATE_SPEC: &str = include_str!("input/spec.md");
pub const AH_TEMPLATE_PLAN: &str = include_str!("input/plan.md");

pub const PROMPT_BUILD_ORIENT: &str = include_str!("input/prompt.phase.orient.md");
pub const PROMPT_BUILD_EXECUTE: &str = include_str!("input/prompt.phase.execute.md");
pub const PROMPT_BUILD_VERIFY: &str = include_str!("input/prompt.phase.verify.md");
pub const PROMPT_PHASE_RECOVER: &str = include_str!("input/prompt.phase.recover.md");

pub const DEMO_COMPLETE_DELAY: Duration = Duration::from_secs(1);
pub const DEMO_EVALUATION_TIME: Duration = Duration::from_secs(1);
pub const DEMO_EXECUTION_TIME: Duration = Duration::from_secs(1);

pub const FMT_HUMAN_READABLE: &'static str = "%B %-d, %Y at %-I:%M:%S %p UTC";
pub const MAX_STAGE_ATTEMPTS: u32 = 100;
pub const STATUS_INTERVAL: Duration = Duration::from_secs(30);

pub static DEFAULT_LOOP_TRIES: u32 = 3;
pub static DEFAULT_NAUL_BAR: f64 = 0.40;
