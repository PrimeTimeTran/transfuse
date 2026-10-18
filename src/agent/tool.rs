use super::*;
// use crate::{native::poc::McpClient};

use Error;
use std::os::unix::fs::MetadataExt;

impl FileSystemTool {
	pub fn search(&self, query: &str) -> Result<Vec<FileInfo>> {
		self.vfs.search_files(query)
	}
	pub fn read(&self, path: &str) -> Result<String> {
		self.vfs.read_file(path)
	}
	pub fn write(&self, path: &str, content: &str) -> Result<()> {
		self.vfs.write_file(path, content)
	}
	pub fn create(&self, path: &str, content: &str) -> Result<()> {
		self.vfs.create_file(path, content)
	}
}
impl FileInfo {
	pub fn from_path(path: &Path) -> Result<Self> {
		let metadata = std::fs::symlink_metadata(path)?;
		let is_directory = metadata.is_dir();
		let content = if metadata.is_file() {
			std::fs::read_to_string(path).unwrap_or_default()
		} else {
			String::new()
		};
		let extension = path
			.extension()
			.and_then(|ext| ext.to_str())
			.map(String::from);
		let name = path
			.file_name()
			.and_then(|name| name.to_str())
			.unwrap_or_default()
			.to_string();
		let path = path.to_string_lossy().into_owned();
		let inode = metadata.ino().to_string();
		let language = extension.as_deref().and_then(language_from_extension);
		Ok(Self {
			content,
			extension,
			inode,
			is_directory,
			language,
			modified_at: metadata.modified().ok(),
			name,
			path,
			size: metadata.len(),
		})
	}
}
impl ShellTool {
	pub async fn run(&self, command: ShellCommand) -> Result<ShellResult> {
		let ShellCommand {
			program,
			args,
			cwd,
			timeout,
		} = command;
		let working_dir = cwd.unwrap_or(std::env::current_dir()?);
		let mut process = tokio::process::Command::new(&program);
		process
			.args(&args)
			.current_dir(&working_dir)
			.stdout(std::process::Stdio::piped())
			.stderr(std::process::Stdio::piped());
		let mut child = process
			.spawn()
			.with_context(|| format!("failed to spawn `{program} {}`", args.join(" ")))?;
		let stdout = child.stdout.take();
		let stderr = child.stderr.take();
		let stdout_task = tokio::spawn(async move {
			if let Some(stdout) = stdout {
				use tokio::io::AsyncReadExt;
				let mut bytes = Vec::new();
				let mut reader = stdout;
				reader.read_to_end(&mut bytes).await?;
				Ok::<_, std::io::Error>(bytes)
			} else {
				Ok(Vec::new())
			}
		});
		let stderr_task = tokio::spawn(async move {
			if let Some(stderr) = stderr {
				use tokio::io::AsyncReadExt;
				let mut bytes = Vec::new();
				let mut reader = stderr;
				reader.read_to_end(&mut bytes).await?;
				Ok::<_, std::io::Error>(bytes)
			} else {
				Ok(Vec::new())
			}
		});

		let status = if let Some(timeout) = timeout {
			match tokio::time::timeout(timeout, child.wait()).await {
				Ok(result) => result?,
				Err(_) => {
					child.kill().await?;
					child.wait().await.ok();
					return Err(anyhow::anyhow!(
						"command timed out after {:?}: {} {}",
						timeout,
						program,
						args.join(" ")
					));
				}
			}
		} else {
			child.wait().await?
		};
		let stdout = stdout_task.await??;
		let stderr = stderr_task.await??;
		Ok(ShellResult {
			program,
			args,
			cwd: working_dir,
			exit_code: status.code(),
			stdout: String::from_utf8_lossy(&stdout).into_owned(),
			stderr: String::from_utf8_lossy(&stderr).into_owned(),
		})
	}
}
impl Vfs {
	pub fn new() -> Self {
		Self
	}
	pub fn search_files(&self, query: &str) -> Result<Vec<FileInfo>, Error> {
		dbg!("Search filesystem for files matching query {}", query);
		Ok(Vec::new())
	}
	pub fn create_file(&self, path: &str, content: &str) -> Result<(), Error> {
		dbg!("Create file: {}", path);
		let path = Path::new(path);
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		std::fs::write(path, content)?;
		println!("Created file: {} ({} bytes)", path.display(), content.len());
		Ok(())
	}

	pub fn read_file(&self, path: &str) -> Result<String, Error> {
		dbg!("Read file: {}", path);

		Ok(std::fs::read_to_string(path)?)
	}

	pub fn write_file(&self, path: &str, content: &str) -> Result<(), Error> {
		dbg!("Write file: {}", path);
		let path = Path::new(path);
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		std::fs::write(path, content)?;
		println!("Wrote file: {} ({} bytes)", path.display(), content.len());
		Ok(())
	}
}

#[derive(Debug, Default, Clone)]
pub struct AgentTools {
	pub fs: FileSystemTool,
	pub shell: ShellTool,
	pub mcp: McpClient,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct FileInfo {
	pub content: String,
	pub extension: Option<String>,
	pub inode: String,
	pub is_directory: bool,
	pub language: Option<String>,
	pub modified_at: Option<SystemTime>,
	pub name: String,
	pub path: String,
	pub size: u64,
}
#[derive(Default, Debug, Clone)]
pub struct FileSystemTool {
	pub vfs: Vfs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
	pub program: String,
	pub args: Vec<String>,
	pub cwd: Option<PathBuf>,
	pub timeout: Option<Duration>,
}
impl ShellCommand {
	pub fn shell(command: impl Into<String>) -> Self {
		let command = command.into();
		section!("COMMAND");
		eprintln!("{command:?}");
		Self {
			program: "sh".into(),
			args: vec!["-c".into(), command],
			cwd: None,
			timeout: None,
		}
	}
	pub fn program(
		program: impl Into<String>,
		args: impl IntoIterator<Item = impl Into<String>>,
	) -> Self {
		let program = program.into();
		let args = args.into_iter().map(Into::into).collect::<Vec<_>>();

		section!("PROGRAM");
		eprintln!("program: {program:?}");
		eprintln!("args: {args:?}");

		Self {
			program,
			args,
			cwd: None,
			timeout: None,
		}
	}
}
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ShellResult {
	pub program: String,
	pub args: Vec<String>,
	pub cwd: PathBuf,
	pub exit_code: Option<i32>,
	pub stdout: String,
	pub stderr: String,
}
#[derive(Debug, Default, Clone)]
pub struct ShellTool;

#[derive(Debug, Default, Clone)]
pub struct Vfs;
