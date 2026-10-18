use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum AgentAction {
	#[serde(rename = "finish")]
	Finish { message: String },

	#[serde(rename = "current")]
	Current { message: String },

	#[serde(rename = "run_command")]
	RunCommand { command: String },

	#[serde(rename = "run_program")]
	RunProgram {
		program: String,
		args: Vec<String>,
		#[serde(default)]
		cwd: Option<String>,
	},

	#[serde(rename = "context")]
	Context { path: Option<String> },
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum AgentEvent {
	NewTask { task: AgentTask },
	Thinking { task: AgentTask },
	Working { task: AgentTask, message: String },
	Finished { result: TaskResult },
	TaskEvent { task: AgentTask, event: TaskEvent },
}

#[derive(Debug)]
pub enum AgentMode {
	Chat,
	Tool,
}
#[derive(PartialEq, Clone)]
pub enum AgentStatus {
	Done,
	Waiting,
	Thinking,
	Error(String),
}
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum AgentObservation {
	Current { message: String },
	RunCommand { result: ShellResult },
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum Artifact {
	FileRead { path: String, content: String },
	FileWrite { path: String },
	Observation(String),
	ToolOutput(String),
}
pub enum CommandSpec {
	Shell { command: String },
	Program { program: String, args: Vec<String> },
}
#[derive(Clone, Debug)]
pub enum ExecutionOutcome {
	Succeeded,
	Failed,
	Rejected { reason: String },
}

#[derive(Debug)]
pub enum GuardDecision {
	Allow,
	Reject(String),
}

#[derive(Clone, Debug)]
pub enum RuntimeEvent {
	Agent(AgentEvent),
	System(SystemEvent),
}

#[derive(Debug, Clone)]
pub enum SystemEvent {
	SpawnAgent { agent_id: String },
	TaskAdd { task_id: String },
	TaskSpawned { task: AgentTask },
	TaskQueued { task_id: String },
	TaskStarted { task_id: String },
	TaskCompleted { result: TaskResult },
	TaskFailed { task_id: String, error: String },
	AgentSpawned { agent_id: String },
	AgentFinished { agent_id: String },
	TaskGroupFinished { group_id: String },
	AllIdle,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum TaskEvent {
	Thinking,
	Started,
	Log(String),
	Working(String),
	Finished(String),
	Error(String),
}

/// ## [TaskStatus]
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
	Pending,
	Running,
	Completed,
	Failed(String),
	Stopped,
	Interrupted,
}
