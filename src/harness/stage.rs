use super::*;
use crate::prelude::*;

impl Stage {
	pub fn next(&self) -> Option<Self> {
		match self {
			Self::Intent => Some(Self::Spec),
			Self::Spec => Some(Self::Plan),
			Self::Plan => Some(Self::Build),
			Self::Test => Some(Self::Build),
			Self::Build => Some(Self::QA),
			Self::QA => Some(Self::Complete),
			Self::Deploy => Some(Self::Maintain),
			Self::Maintain => Some(Self::Complete),
			Self::Complete => None,
			Self::Finalize => None,
		}
	}
	pub fn is_before(self, other: Stage) -> bool {
		let rank = |stage: Stage| match stage {
			Stage::Intent => 0,
			Stage::Spec => 1,
			Stage::Plan => 2,
			Stage::Test => 3,
			Stage::Build => 4,
			Stage::QA => 5,
			Stage::Deploy => 6,
			Stage::Maintain => 7,
			Stage::Complete => 8,
			Stage::Finalize => DEFAULT_LOOP_TRIES,
		};
		rank(self) < rank(other)
	}
}
impl std::fmt::Display for Stage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let name = match self {
			Self::Intent => "Intent",
			Self::Spec => "Spec",
			Self::Plan => "Plan",
			Self::Test => "Test",
			Self::Build => "Build",
			Self::QA => "QA",
			Self::Deploy => "Deploy",
			Self::Maintain => "Maintain",
			Self::Complete => "Complete",
			Self::Finalize => "Finalize Sprint",
		};

		f.write_str(name)
	}
}

pub struct StageIntent {
	pub name: String,
	pub description: String,

	pub problem: Option<String>,
	pub motivation: Option<String>,
	pub desired_outcome: Option<String>,

	pub scope: Option<Vec<String>>,
	pub out_of_scope: Option<Vec<String>>,
	pub constraints: Option<Vec<String>>,

	pub success_criteria: Option<Vec<String>>,
}

pub struct StageSpec {
	pub name: String,
	pub description: String,

	pub requirements: Vec<Requirement>,
	pub acceptance_criteria: Vec<AcceptanceCriterion>,

	pub constraints: Option<Vec<String>>,
	pub edge_cases: Option<Vec<String>>,
	pub unresolved_questions: Option<Vec<String>>,
}

pub struct Requirement {
	pub id: String,
	pub description: String,
	pub priority: Option<String>,
}

pub struct AcceptanceCriterion {
	pub id: String,
	pub description: String,
	pub requirement_ids: Vec<String>,
}
pub struct StagePlan {
	pub name: String,
	pub description: String,

	pub framework: Option<String>,
	pub language: Option<String>,

	pub test_suite_lang: Option<String>,
	pub test_suite_framework: Option<String>,

	pub targets: Option<Vec<String>>,

	pub steps: Vec<PlanStep>,
	pub dependencies: Option<Vec<String>>,

	pub verification_strategy: Option<Vec<String>>,
	pub risks: Option<Vec<String>>,
}
pub struct PlanStep {
	pub id: String,
	pub description: String,
	pub target_files: Option<Vec<String>>,
	pub requirement_ids: Vec<String>,
}

pub struct StageBuild {
	pub name: String,
	pub description: String,

	pub framework: Option<String>,
	pub language: Option<String>,

	pub changed_files: Vec<String>,
	pub completed_steps: Vec<String>,
	pub remaining_steps: Vec<String>,

	pub executions: Vec<ExecutionRecord>,
	pub errors: Vec<String>,
}
