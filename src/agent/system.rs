use tokio::sync::mpsc::UnboundedReceiver;

use super::*;
use crate::prelude::*;

pub async fn handle_event(event: AgentEvent, runtime: &AgentRuntime) {
	match event {
		AgentEvent::NewTask { task } => {
			runtime.spawn_agent(task).await;
		}
		AgentEvent::Finished { result } => {
			let _ = runtime
				.event_tx
				.send(RuntimeEvent::System(SystemEvent::TaskCompleted { result }));
		}
		_ => {}
	}
}
pub fn new_agent_system() -> (AgentBus, AgentRuntime, UnboundedReceiver<RuntimeEvent>) {
	let (cmd_tx, _cmd_rx) = unbounded_channel::<AgentEvent>();
	let (event_tx, event_rx) = unbounded_channel::<RuntimeEvent>();
	let bus = AgentBus {
		tx: cmd_tx,
		event_tx: event_tx.clone(),
	};
	let runtime = AgentRuntime {
		event_tx,
		workspace: CtxWorkspace::default(),
		registry: AgentRegistry::default(),
	};
	(bus, runtime, event_rx)
}

impl AgentSystem {
	pub fn add_file(&mut self, path: impl Into<PathBuf>) -> Result<()> {
		self.runtime.workspace.add_file(path)
	}
	pub fn new() -> Self {
		let (cmd_tx, _cmd_rx) = unbounded_channel::<AgentEvent>();
		let (event_tx, event_rx) = unbounded_channel::<RuntimeEvent>();
		let bus = AgentBus {
			tx: cmd_tx,
			event_tx: event_tx.clone(),
		};
		let runtime = AgentRuntime {
			event_tx,
			workspace: CtxWorkspace::default(),
			registry: AgentRegistry::default(),
		};
		Self {
			bus,
			runtime,
			event_rx,
			ctx: AgentCtx::init(),
		}
	}
	pub fn cwd(&mut self, cwd: impl Into<PathBuf>) -> &mut Self {
		self.runtime.workspace.cwd = cwd.into();
		self
	}
}
impl CtxWorkspace {
	pub fn from_session(session: &PLSession) -> Result<Self> {
		let mut files = Vec::new();
		for entry in std::fs::read_dir(&session.dir)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_file() {
				files.push(FileInfo::from_path(&path)?);
			}
		}
		Ok(Self {
			files,
			files_wip: Vec::new(),
			files_todo: Vec::new(),
			cwd: session.dir.clone(),
		})
	}
	pub fn add_file(&mut self, path: impl Into<PathBuf>) -> Result<()> {
		let path = path.into();

		if self.files.iter().any(|file| file.path == path) {
			return Ok(());
		}

		let file = FileInfo::from_path(&path)?;
		self.files.push(file);

		Ok(())
	}
	pub fn from_cwd(cwd: impl Into<PathBuf>) -> Self {
		Self {
			cwd: cwd.into(),
			files: Vec::new(),
			files_wip: Vec::new(),
			files_todo: Vec::new(),
		}
	}
}
impl Default for CtxWorkspace {
	fn default() -> Self {
		Self {
			files: Vec::new(),
			files_wip: Vec::new(),
			files_todo: Vec::new(),
			cwd: std::env::current_dir().unwrap_or_default(),
		}
	}
}
impl Display for CtxWorkspace {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "CWD: {}", self.cwd.display())?;
		writeln!(f, "FILES: {}", self.files.len())?;
		if self.files.is_empty() {
			return Ok(());
		}
		for (i, file) in self.files.iter().take(5).enumerate() {
			writeln!(f, "  [{i}] {file:?}")?;
		}
		if self.files.len() > 5 {
			writeln!(f, "  ... {} more files", self.files.len() - 5)?;
		}
		Ok(())
	}
}

#[derive(Debug)]
pub struct AgentSystem {
	pub bus: AgentBus,
	pub runtime: AgentRuntime,
	pub event_rx: UnboundedReceiver<RuntimeEvent>,
	pub ctx: AgentCtx,
}
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CtxWorkspace {
	pub files: Vec<FileInfo>,
	pub files_wip: Vec<FileInfo>,
	pub files_todo: Vec<FileInfo>,
	pub cwd: PathBuf,
}
