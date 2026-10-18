use super::*;
use crate::prelude::*;

pub fn build_steps() -> Vec<&'static str> {
	vec![
		"Read plan.md and identify the files that must be created or modified.",
		"Inspect the relevant existing files and repository structure for the files identified by the plan.",
		"Create the planned files and establish their basic structure.",
		"Implement the planned functionality in the files created or modified so far.",
		"Inspect the implementation and compare it against the specification and plan.",
		"Add the planned unit and integration tests.",
		"Run the relevant tests and verification commands.",
		"Inspect any failures and determine what implementation changes are required.",
		"Fix the implementation or tests based on the failures and rerun verification.",
		"Review the completed implementation against the plan and identify any remaining work.",
	]
}
pub fn build_step_prompt(
	instruction: &str,
	step: usize,
	total: usize,
	plan: &str,
	workspace: &str,
) -> String {
	format!(
		r#"
      You are executing BUILD STEP {step}/{total}.

      YOUR CURRENT TASK:
      {instruction}

      ---

      IMPLEMENTATION PLAN:
      {plan}

      ---

      CURRENT AGENT CONTEXT:
      {workspace}

      ---

      EXECUTION RULES:

      1. Start by reasoning from the task, implementation plan, and agent context
         already provided above. Do not retrieve information that is already
         available to you.

      2. Use the `context` action when you need to inspect the actual contents
         of a file or artifact provided through agent context, or when you need
         additional curated context that was not included in the prompt.

      3. Use `run_command` when you need to inspect or modify the actual
         filesystem, repository, source code, build system, tests, or other
         external state.

      4. Do not use filesystem commands merely to locate or read a file that is
         already available through agent context. Use `context` for that.

      5. Before modifying existing source files, inspect the relevant files from
         the actual repository using `run_command` as necessary.

      6. Perform the work directly in the workspace.

      7. Do not merely describe what should be done. Take the concrete action.

      8. Complete only the current build step.

      9. Preserve existing project conventions and correct work from previous
         build steps.

      10. When this step is complete, stop and report the result.

      ---

      DECISION ORDER:

      Information already provided
          ↓
      context (only if additional curated file contents are needed)
          ↓
      run_command (when actual repository/filesystem interaction is needed)
          ↓
      make the required changes
          ↓
      verify the result
          ↓
      finish
    "#,
	)
}

pub fn duration_readable(duration: chrono::Duration) -> String {
	let millis = duration.num_milliseconds();
	if millis < 1000 {
		return format!("{millis} ms");
	}
	let seconds = millis / 1000;
	let hours = seconds / 3600;
	let minutes = (seconds % 3600) / 60;
	let seconds = seconds % 60;
	match (hours, minutes, seconds) {
		(h, m, _) if h > 0 => format!("{h}h {m}m"),
		(_, m, s) if m > 0 => format!("{m}m {s}s"),
		(_, _, s) => format!("{s}s"),
	}
}
pub fn format_elapsed(duration: std::time::Duration) -> String {
	let total_seconds = duration.as_secs();
	let hours = total_seconds / 3600;
	let minutes = (total_seconds % 3600) / 60;
	let seconds = total_seconds % 60;

	if hours > 0 {
		format!("{hours}h {minutes}m {seconds}s")
	} else if minutes > 0 {
		format!("{minutes}m {seconds}s")
	} else {
		format!("{seconds}s")
	}
}

pub fn git_status() -> Result<String> {
	let output = std::process::Command::new("git")
		.args(["status", "--short"])
		.output()?;
	if !output.status.success() {
		return Err(anyhow::anyhow!(
			"git status failed: {}",
			String::from_utf8_lossy(&output.stderr)
		));
	}
	Ok(String::from_utf8(output.stdout)?)
}
pub fn persist_evaluation(
	session: &mut PLSession,
	evaluation: &QACheck,
	attempt: Attempt,
) -> Result<()> {
	let status = if evaluation.passed {
		PLStatus::Completed
	} else {
		PLStatus::NeedsRevision
	};
	let record = StageRunRecord {
		description: Some(String::from("Evaluation Complete")),
		actor: StageActor::Evaluator,
		attempt,
		evaluation: Some(evaluation.clone()),
		stage: evaluation.stage,
		time_started: evaluation.time_started,
		time_completed: Some(Utc::now()),
		status,
	};
	let _time_started = evaluation.time_started;
	let _time_completed = Utc::now();
	session.stages.push(record);
	session.time_updated = Utc::now();
	Ok(())
}
pub fn time_readable(time: chrono::DateTime<chrono::Utc>) -> String {
	time.format("%B %-d, %Y at %-I:%M:%S %p UTC").to_string()
}
pub fn log_step_transition(from: Stage, to: Stage) -> Result<()> {
	let path: PathBuf = env::current_dir()?.join("current_step.txt");
	let mut file = OpenOptions::new().create(true).append(true).open(path)?;
	writeln!(file, "{:?} -> {:?}", from, to)?;
	Ok(())
}

pub fn ah_log_file(name: &str) -> PathBuf {
	PathBuf::from(AH_LOG_DIR).join(name)
}
pub fn short_duration_readable(duration: chrono::Duration) -> String {
	let seconds = duration.num_seconds();
	let hours = seconds / 3600;
	let minutes = (seconds % 3600) / 60;
	let seconds = seconds % 60;

	match (hours, minutes, seconds) {
		(h, m, _) if h > 0 => format!("{h} hours {m} minutes"),
		(_, m, s) if m > 0 => format!("{m} minutes {s} seconds"),
		(_, _, s) => format!("{s} seconds"),
	}
}
pub fn slugify(input: &str) -> String {
	let slug = input
		.to_lowercase()
		.chars()
		.map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
		.collect::<String>();
	let slug = slug
		.split('-')
		.filter(|s| !s.is_empty())
		.take(8)
		.collect::<Vec<_>>()
		.join("-");
	if slug.is_empty() {
		return "untitled".into();
	}
	// Windows reserved device names.
	let reserved = [
		"con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
		"com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
	];
	if reserved.contains(&slug.as_str()) {
		format!("session-{slug}")
	} else {
		slug
	}
}

pub async fn monitor<F, T>(
	_stage: Stage,
	_attempt: u32,
	_run_started: Instant,
	_phase: &'static str,
	future: F,
) -> T
where
	F: Future<Output = T>,
{
	let _started = Instant::now();
	tokio::pin!(future);
	let mut ticker = tokio::time::interval(STATUS_INTERVAL);
	loop {
		tokio::select! {
			result = &mut future => {
				return result;
			}
			_ = ticker.tick() => {
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct BuildStep {
	pub instruction: &'static str,
	pub completion: &'static str,
}

impl BuildStep {
	pub const fn new(instruction: &'static str, completion: &'static str) -> Self {
		Self {
			instruction,
			completion,
		}
	}
}

#[derive(Debug, Clone)]
pub struct BuildPlan {
	pub goal: String,
	pub files: Vec<BuildFile>,
	pub steps: Vec<BuildStep>,
}

#[derive(Debug, Clone)]
pub struct BuildFile {
	pub path: PathBuf,
	pub action: BuildFileAction,
}

#[derive(Debug, Clone, Copy)]
pub enum BuildFileAction {
	Create,
	Modify,
}
impl BuildPlan {
	pub fn new(goal: impl Into<String>, steps: Vec<BuildStep>) -> Self {
		Self {
			files: vec![],
			goal: goal.into(),
			steps,
		}
	}
	pub fn len(&self) -> usize {
		self.steps.len()
	}
	pub fn is_empty(&self) -> bool {
		self.steps.is_empty()
	}
	pub fn step(&self, index: usize) -> Option<&BuildStep> {
		self.steps.get(index)
	}
}
// pub fn build_plan() -> BuildPlan {
// 	BuildPlan::new(
// 		"Implement the requested functionality according to the specification and plan.",
// 		vec![
// 			BuildStep::new(
// 				"Read plan.md and identify the files that must be created or modified.",
// 				"All planned files and their intended locations are identified.",
// 			),
// 			BuildStep::new(
// 				"Inspect the relevant existing files and repository structure for the files identified by the plan.",
// 				"The relevant existing files and repository structure have been inspected.",
// 			),
// 			BuildStep::new(
// 				"Create the planned files and establish their basic structure.",
// 				"Every planned file exists at its intended path.",
// 			),
// 			BuildStep::new(
// 				"Implement the planned functionality in the files created or modified so far.",
// 				"The planned functionality is implemented in the intended files.",
// 			),
// 			BuildStep::new(
// 				"Inspect the implementation and compare it against the specification and plan.",
// 				"The implementation has been checked against the specification and plan.",
// 			),
// 			BuildStep::new(
// 				"Add the planned unit and integration tests.",
// 				"All planned tests have been created at their intended paths.",
// 			),
// 			BuildStep::new(
// 				"Run the relevant tests and verification commands.",
// 				"The relevant tests and verification commands have actually been executed and their results are recorded.",
// 			),
// 			BuildStep::new(
// 				"Inspect any failures and determine what implementation changes are required.",
// 				"Each failure has been understood and a corrective action has been identified.",
// 			),
// 			BuildStep::new(
// 				"Fix the implementation or tests based on the failures and rerun verification.",
// 				"Previously failing verification passes, or any remaining failure is explicitly understood.",
// 			),
// 			BuildStep::new(
// 				"Review the completed implementation against the plan and identify any remaining work.",
// 				"No required work from the plan remains incomplete.",
// 			),
// 		],
// 	)
// }
