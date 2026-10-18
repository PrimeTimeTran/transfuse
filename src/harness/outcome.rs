use super::*;

#[derive(Debug)]
pub enum Outcome {
	Complete {
		execution: Execution,
		evaluation: QACheck,
	},
	NeedsRevision {
		execution: Execution,
		evaluation: QACheck,
	},
	ExecutionFailed {
		stage: Stage,
		attempt: Attempt,
		error: anyhow::Error,
	},
	EvaluationFailed {
		execution: Execution,
		error: anyhow::Error,
	},
}

#[derive(Debug)]
pub enum AttemptOutcome {
	Progress {
		execution: Execution,
		evaluation: QACheck,
	},
	NeedsRevision {
		execution: Execution,
		evaluation: QACheck,
	},
	ExecutionFailed {
		stage: Stage,
		attempt: Attempt,
		error: anyhow::Error,
	},
	EvaluationFailed {
		execution: Execution,
		error: anyhow::Error,
	},
}
#[derive(Debug)]
pub enum PipelineStatus {
	Complete,
	NeedsRevision,
	Failed,
	Interrupted,
}
#[derive(Debug)]
pub enum StageDisposition {
	Complete,
	NeedsRevision,
	Failed,
}
#[derive(Debug)]
pub struct StageOutcome {
	pub stage: Stage,
	pub attempts: Vec<AttemptOutcome>,
	pub disposition: StageDisposition,
}
#[derive(Debug)]
pub struct PipelineOutcome {
	pub status: PipelineStatus,
	pub stages: Vec<StageOutcome>,
}
