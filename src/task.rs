use crate::prelude::*;

impl TaskResult {
	pub fn completed_chat(task_id: Uuid, ctx: AgentCtx, chat: String) -> Self {
		Self {
			artifacts: ctx.artifacts,
			chat: Some(chat),
			logs: ctx.logs,
			spawned_tasks: ctx.spawned_tasks,
			status: TaskStatus::Completed,
			summary: None,
			task_id,
			commands: Vec::new(),
			errors: Vec::new(),
		}
	}
	pub fn completed_with_summary(
		task_id: Uuid,
		ctx: AgentCtx,
		summary: String,
		commands: Vec<CommandRecord>,
		errors: Vec<AgentError>,
	) -> Self {
		Self {
			artifacts: ctx.artifacts,
			chat: None,
			logs: ctx.logs,
			spawned_tasks: ctx.spawned_tasks,
			status: TaskStatus::Completed,
			summary: Some(summary),
			task_id,
			commands,
			errors,
		}
	}
	pub fn failed(
		task_id: Uuid,
		ctx: AgentCtx,
		reason: impl Into<String>,
		summary: Option<String>,
		commands: Vec<CommandRecord>,
		errors: Vec<AgentError>,
	) -> Self {
		Self {
			artifacts: ctx.artifacts,
			chat: None,
			logs: ctx.logs,
			spawned_tasks: ctx.spawned_tasks,
			status: TaskStatus::Failed(reason.into()),
			summary,
			task_id,
			commands,
			errors,
		}
	}
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskResult {
	pub artifacts: Vec<Artifact>,
	pub chat: Option<String>,
	pub logs: Vec<String>,
	pub spawned_tasks: Vec<AgentTask>,
	pub status: TaskStatus,
	pub summary: Option<String>,
	pub task_id: Uuid,
	#[serde(default)]
	pub commands: Vec<CommandRecord>,
	#[serde(default)]
	pub errors: Vec<AgentError>,
}
