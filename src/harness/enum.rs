use super::*;
use crate::prelude::*;

pub enum ArtifactPolicy {
	/// Require valid upstream artifacts before executing.
	RequireValid,
	/// Explicitly allow existing artifacts, even if QA previously rejected them.
	AcceptExisting,
	/// Regenerate missing or invalid upstream artifacts as needed.
	Regenerate,
}

#[derive(Debug)]
pub enum Decision {
	Continue,
	Retry,
	Revise,
	AwaitHuman,
	Fail,
	Complete,
	Exit,
}

#[derive(Debug)]
pub enum ExecutionResult {
	Completed(TaskResult),
	Failed(StageError),
}

pub enum InputMode {
	Normal,
	Human { buffer: String },
	AwaitingHuman { prompt: String },
}
#[derive(Debug, Clone)]
pub enum Intervention {
	Human(String),
	Retry,
	ProvideContext(String),
	Reviewed,
	Abort,
	Revise,
	Revision { evaluation: QACheck },
}

#[derive(Debug, Clone, Copy)]
pub enum GenerationProvider {
	Local,
	Api,
}

#[derive(Debug, Clone, Deserialize, Serialize, Copy, PartialEq)]
pub enum Phase {
	Starting,
	Executing,
	Evaluating,
	Retrying,
	AwaitingHuman,
	Completed,
	Failed,
}
pub type PipelineId = uuid::Uuid;

#[derive(Debug)]
pub enum RunControl {
	Continue,
	RetryStage,
	Exit(PipelineStatus),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SdlcEvent {
	ModelPrepare,
	ModelAssess,
	ModelExecute,
	ModelVerify,

	BuildBegin,
	BuildPrepare,
	BuildAssess,
	BuildExecute,
	BuildVerify,

	RunBegin,
	RunExited {
		reason: String,
	},
	RunAttempt {
		stage: Stage,
		phase: Phase,
	},
	Activity {
		stage: Stage,
		attempt: Attempt,
		message: String,
	},
	Completed,
	Failed {
		stage: Option<Stage>,
		error: String,
	},
	HumanInput {
		stage: Stage,
		input: String,
	},
	InterventionRequired {
		stage: Stage,
		attempt: Attempt,
		reason: String,
	},
	InterventionResolved {
		stage: Stage,
		action: String,
	},
	Evaluated {
		stage: Stage,
		score: f64,
		confidence: f64,
		passed: bool,
	},
	EvaluationStarted {
		stage: Stage,
	},
	EvaluationFailed {
		stage: Stage,
		error: String,
	},
	ExecutionComplete {
		stage: Stage,
	},
	ExecutionFailed {
		stage: Stage,
		attempt: Attempt,
		error: String,
	},
	PhaseChanged {
		stage: Stage,
		phase: Phase,
	},
	// PhaseAttempt {
	// 	stage: Stage,
	// 	phase: Phase,
	// },
	// StageRetrying {
	// 	stage: Stage,
	// 	number: u32,
	// },
	StageRunRetry {
		stage: Stage,
		number: u32,
	},
	StageStarted {
		stage: Stage,
		attempt: Attempt,
	},
	StageTransitioned {
		from: Stage,
		to: Stage,
	},
}
#[derive(Debug)]
pub enum SdlcInput {
	Abort,
	ProvideContext(String),
	Retry,
	Reviewed,
	Revision { evaluation: QACheck },
	Human(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
	Intent,
	Spec,
	Plan,
	Test,
	Build,
	QA,
	Deploy,
	Maintain,
	Complete,
	Finalize,
}

#[derive(Debug, Clone, Copy)]
pub enum StageAction {
	Continue,
	Retry,
	Intervene,
	Abort,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StageActor {
	Human,
	Runner,
	Evaluator,
	Agent,
	System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageInput {
	Initial,
	Revision { evaluation: QACheck },
}

pub enum StageOutcomeEvaluation {
	Passed(Evaluation),
	FailedQuality(Evaluation),
	FailedRuntime(Error),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RunResult {
	Intent,
	Spec,
	Plan,
	Build,
	Verification(Verification),
	Complete,
	Finalize,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PLStatus {
	Running,
	Completed,
	Failed,
	NeedsRevision,
	EvaluationFailed,
	InterventionNeeded,
}

#[derive(Debug)]
pub enum RunResults {
	Pipeline(PipelineOutcome),
	Stage(StageOutcome),
	Attempt(AttemptOutcome),
}

#[derive(Debug)]
pub enum RunStrategy {
	Full,
	ResumeFrom(Stage),
	Stage(Stage),
	SingleAttempt(Stage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
	Boot,
	Init,
	Intent,
	Spec,
	Plan,
	Build,
	Verify,
	Deploy,
	Maintain,
	Complete,
}

pub enum Transition {
	Next,
	Repeat,
	Goto(Stage),
	Complete,
	Fail,
	AwaitHuman,
}
