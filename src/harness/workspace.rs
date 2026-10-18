use super::*;
use crate::prelude::*;

impl WSChanges {
	pub fn file_count(&self) -> usize {
		self
			.git_status
			.lines()
			.filter(|line| !line.trim().is_empty())
			.count()
	}
	pub fn to_markdown(&self, agent_result: &str) -> String {
		let mut markdown = String::from("# Build\n\n");
		markdown.push_str("## Workspace Changes\n\n");
		if self.git_status.trim().is_empty() {
			markdown.push_str("No Git changes detected.\n");
		} else {
			markdown.push_str("```text\n");
			markdown.push_str(&self.git_status);
			markdown.push_str("```\n");
		}
		markdown.push_str("\n## Agent Result\n\n");
		markdown.push_str(agent_result);
		markdown.push('\n');
		markdown
	}
}
impl WSSnapshot {
	pub fn capture(workspace: impl AsRef<Path>) -> Result<Self> {
		let workspace = workspace.as_ref();
		let output = std::process::Command::new("git")
			.args(["status", "--short", "--porcelain=v1"])
			.current_dir(workspace)
			.output()
			.with_context(|| format!("failed to capture git status in {}", workspace.display()))?;
		if !output.status.success() {
			return Err(anyhow::anyhow!(
				"git status failed: {}",
				String::from_utf8_lossy(&output.stderr).trim()
			));
		}
		Ok(Self {
			git_status: String::from_utf8(output.stdout)
				.context("git status output was not valid UTF-8")?,
		})
	}
	pub fn diff(&self, after: &Self) -> WSChanges {
		WSChanges {
			git_status: after.git_status.clone(),
		}
	}
	pub fn to_markdown(&self) -> String {
		format!(
			"## Workspace State\n\n\
             **Git status:**\n\n\
             ```text\n\
             {}\n\
             ```\n",
			self.git_status
		)
	}
}
