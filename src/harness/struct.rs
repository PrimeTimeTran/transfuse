use super::*;
use crate::prelude::{SdlcEvent as Event, *};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PLSession {
	pub id: Uuid,
	pub title: String,
	pub prompt: String,
	pub stage: Stage,
	pub stages: Vec<StageRunRecord>,
	pub dir: PathBuf,
	pub time_created: DateTime<Utc>,
	pub time_updated: DateTime<Utc>,
	pub workspace: PathBuf,
	pub cwd: PathBuf,
}
pub struct AiView {
	pub runtime: PLRuntimeSS,
	pub paused: bool,
	pub show_logs: bool,
	pub input: String,
	pub input_active: bool,
	pub events: Vec<Event>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Attempt {
	pub stage: Stage,
	pub number: u32,
	pub max: u32,
}
#[derive(Clone, Debug)]
pub struct ApiGenerator {
	// whatever API client you decide to use
}

pub struct BuildResult {
	pub changed_files: Vec<PathBuf>,
	pub created_files: Vec<PathBuf>,
	pub modified_files: Vec<PathBuf>,
	pub deleted_files: Vec<PathBuf>,
	pub commands: Vec<CmddResult>,
	pub agent_summary: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildPhaseEvent {
	pub phase: String,
	pub status: String,
	pub elapsed_ms: Option<u128>,
	pub error: Option<String>,
}

pub struct Check {
	pub name: String,
	pub command: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
	pub name: String,
	pub passed: bool,
	pub output: Option<String>,
}
pub struct CmddResult {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult;
pub struct CtxEvaluation {
	pub stage: Stage,
	pub intent: Option<String>,
	pub spec: Option<String>,
	pub plan: Option<String>,
	pub tests: Option<String>,
	pub progress: Option<String>,
	pub verification: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Evaluator {
	pub jev: TypeSafeClient,
	pub session: PLSession,
}
#[derive(Debug, Clone)]
pub struct Evaluation {
	pub score: f64,
	pub confidence: f64,
	pub meets_bar: bool,
	pub feedback: String,
	pub criteria: Vec<CriterionResult>,
}

#[derive(Debug)]
pub struct Execution {
	pub stage: Stage,
	pub attempt: Attempt,
	pub time_started: chrono::DateTime<Utc>,
	pub time_completed: chrono::DateTime<Utc>,
	pub result: RunResult,
}
#[derive(Clone, Debug)]
pub struct LocalGenerator {
	pub runtime: AgentRuntime,
	pub model: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
	pub name: String,
	pub passed: bool,
	pub score: f64,
	pub confidence: f64,
	pub explanation: String,
}
#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
	pub model: String,
	pub response: String,
	pub done: bool,
	pub done_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QACheck {
	pub confidence: f64,
	pub passed: bool,
	pub score: f64,
	pub stage: Stage,
	pub time_completed: DateTime<Utc>,
	pub time_started: DateTime<Utc>,
	pub time_total: chrono::Duration,
	pub evaluations: Vec<Metric>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAEvaluation {
	pub score: f32,
	pub confidence: f32,
	pub passed: bool,
	pub checks: Vec<CheckResult>,
	pub feedback: String,
	pub revision: Option<String>,
}

// Snapshot Pipeline Runtime
#[derive(Debug, Clone)]
pub struct PLRuntimeSS {
	pub activity: Vec<String>,
	pub attempt: u32,
	pub confidence: Option<f64>,
	pub error: Option<String>,
	pub events: Vec<Event>,
	pub history: Vec<Event>,
	pub message: Option<String>,
	pub passed: Option<bool>,
	pub phase: Phase,
	pub score: Option<f64>,
	pub stage: Stage,
	pub stage_time_started: Instant,
	pub time_started: Instant,
	pub total_agent_calls: u32,
	pub total_tokens: u64,
}

#[derive(Debug)]
pub struct StageError;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRunRecord {
	pub attempt: Attempt,
	pub description: Option<String>,
	pub stage: Stage,
	pub status: PLStatus,

	/// What actually performed the work.
	pub actor: StageActor,

	pub time_started: DateTime<Utc>,
	pub time_completed: Option<DateTime<Utc>>,

	/// Semantic evaluation of the resulting artifact/work.
	pub evaluation: Option<QACheck>,
}

pub struct TerminalGuard;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
	pub passed: bool,
	// pub score: u32,
	pub checks: Vec<CheckResult>,
	pub evaluations: Vec<Metric>,
}

#[derive(Debug, Clone)]
pub struct WSChanges {
	pub git_status: String,
}
#[derive(Debug, Clone)]
pub struct WSSnapshot {
	pub git_status: String,
}

pub(crate) struct CtxContext {
	pub stage: Stage,
	pub phase: Phase,
	pub attempt: u32,
	pub session: PLSession,
	pub log_dir: PathBuf,
}
