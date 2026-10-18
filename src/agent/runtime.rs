use anyhow::bail;

use super::*;
use crate::{r#const::AGENT_GEN_URL, prelude::SdlcEvent as E};

impl AgentRuntime {
	pub fn workspace(&self) -> &CtxWorkspace {
		&self.workspace
	}
	pub async fn from_session(&self, task: AgentTask, session: &PLSession) -> Result<TaskResult> {
		let ctx = AgentCtx::from_session_with_workspace(session, &self.workspace)?;
		let agent = Agent::with_ctx(ctx, session)?;
		agent.run_agent_loop(task, self.event_tx.clone()).await
	}
	pub async fn generate(&self, prompt: &str) -> Result<String> {
		if prompt.trim().is_empty() {
			return Err(anyhow!("AgentRuntime generate prompt is empty"));
		}
		// const AGENT_GEN_URL: &str = "http://127.0.0.1:11434/api/generate";
		let request = serde_json::json!({
			"model": DEFAULT_MODEL,
			"prompt": prompt,
			"stream": false,
			"options": {
				"num_ctx": ctx_window_size(),
			},
		});

		std::fs::write(
			ah_log_file("ollama-request.json"),
			serde_json::to_string_pretty(&request)?,
		)
		.context("writing Ollama request debug file")?;

		let started = std::time::Instant::now();

		tracing::info!(url = AGENT_GEN_URL, "Ollama: sending request");

		let response = reqwest::Client::new()
			.post(AGENT_GEN_URL)
			.json(&request)
			.send()
			.await
			.context("sending one-shot Ollama generation request")?;

		tracing::info!(
			elapsed_ms = started.elapsed().as_millis(),
			status = %response.status(),
			"Ollama: response headers received"
		);

		let status = response.status();

		let body = response
			.text()
			.await
			.context("reading Ollama response body")?;

		tracing::info!(
			status = %status,
			body_len = body.len(),
			"Ollama: response body received"
		);

		if !status.is_success() {
			return Err(anyhow!("Ollama HTTP {status}: {body}"));
		}

		std::fs::write(ah_log_file("ollama-response.json"), &body)
			.context("writing Ollama response debug file")?;

		let response: OllamaResponse =
			serde_json::from_str(&body).context("invalid Ollama response")?;

		let generated = response.response.trim();

		if generated.is_empty() {
			return Err(anyhow!(
				"Ollama returned an empty response (done_reason={:?})",
				response.done_reason,
			));
		}

		Ok(generated.to_string())
	}
	pub async fn run_agent(&self, task: AgentTask) -> Result<TaskResult> {
		let agent = Agent::with_workspace(self.workspace.clone());
		agent.run_agent_loop(task, self.event_tx.clone()).await
	}
	pub async fn spawn_agent(&self, task: AgentTask) {
		let event_tx = self.event_tx.clone();
		tokio::spawn(async move {
			let agent = Agent::new();
			let result = agent.run_agent_loop(task.clone(), event_tx.clone()).await;
			match result {
				Ok(result) => {
					let _ = event_tx.send(RuntimeEvent::System(SystemEvent::TaskCompleted {
						result: result.clone(),
					}));
					for task in result.spawned_tasks {
						let _ = event_tx.send(RuntimeEvent::System(SystemEvent::TaskSpawned { task }));
					}
				}
				Err(e) => {
					let _ = event_tx.send(RuntimeEvent::System(SystemEvent::TaskFailed {
						task_id: task.id.to_string(),
						error: e.to_string(),
					}));
				}
			}
		});
	}
}
impl AgentRuntime {
	pub fn new(
		event_tx: UnboundedSender<RuntimeEvent>,
		registry: AgentRegistry,
		workspace: CtxWorkspace,
	) -> Self {
		Self {
			event_tx,
			registry,
			workspace,
			// metrics: RuntimeMetrics::new(),
		}
	}
}

impl AgentRuntime {
	pub async fn assess(&self, plan: &str) -> Result<BuildManifest> {
		log::emit(E::ModelAssess);
		let prompt = format!(
			"{PROMPT_BUILD_ORIENT}\n\n\
			## Implementation Plan\n\n\
			{plan}\n\n\
			Convert this plan into a file-operation manifest.\n\n\
			Return only valid JSON matching this structure:\n\
			{{\"version\":1,\"files\":[{{\"path\":\"src/example.rs\",\
			\"operation\":\"create\",\"reason\":\"Why this change is required\"}}]}}\n\n\
			Each file must have a workspace-relative path, one operation \
			(create, update, or delete), and a concise reason grounded in \
			the plan.\n\
			Include all files required by the plan and no unrelated files.\n\
			Do not implement changes or execute commands.\n\
			Do not return Markdown fences or commentary."
		);
		let response = self.generate_text(prompt).await?;
		let manifest: BuildManifest = serde_json::from_str(response.trim())
			.context("parsing Plan-derived response as BuildManifest")?;
		Self::validate_build_manifest(&manifest, &self.workspace.cwd)?;
		Ok(manifest)
	}

	pub async fn execute(
		&self,
		manifest: &BuildManifest,
		intent: &str,
		spec: &str,
		plan: &str,
	) -> Result<BuildExecutionReport> {
		log::emit(E::ModelExecute);
		Self::validate_build_manifest(manifest, &self.workspace.cwd)?;
		let mut report = BuildExecutionReport {
			files: Vec::with_capacity(manifest.files.len()),
		};
		for file in &manifest.files {
			let path = Self::resolve_build_path(&self.workspace.cwd, &file.path)?;

			let result: Result<String> = async {
				match file.operation {
					BuildOperation::Create => {
						// Recheck immediately before creating.
						if tokio::fs::symlink_metadata(&path).await.is_ok() {
							bail!("Create target already exists: {}", file.path.display());
						}

						let prompt = format!(
							"Generate the complete contents of one new file for \
						 the approved implementation.\n\n\
						 ## Intent\n\n{intent}\n\n\
						 ## Specification\n\n{spec}\n\n\
						 ## Plan\n\n{plan}\n\n\
						 ## Target\n\n{}\n\n\
						 ## Reason\n\n{}\n\n\
						 Return only the file contents. No Markdown fences, \
						 shell commands, or explanatory text.",
							file.path.display(),
							file.reason,
						);

						let content = self.generate_text(prompt).await?;

						if let Some(parent) = path.parent() {
							tokio::fs::create_dir_all(parent)
								.await
								.context("creating parent directories")?;
						}

						let mut output = tokio::fs::OpenOptions::new()
							.write(true)
							.create_new(true)
							.open(&path)
							.await
							.with_context(|| format!("creating {}", file.path.display()))?;

						tokio::io::AsyncWriteExt::write_all(&mut output, content.as_bytes())
							.await
							.context("writing generated content")?;

						Ok("Created file".to_string())
					}
					BuildOperation::Update => {
						let existing = tokio::fs::read_to_string(&path)
							.await
							.with_context(|| format!("reading {}", file.path.display()))?;

						let prompt = format!(
							"Update one existing file to implement the approved plan.\n\n\
						 ## Intent\n\n{intent}\n\n\
						 ## Specification\n\n{spec}\n\n\
						 ## Plan\n\n{plan}\n\n\
						 ## Target\n\n{}\n\n\
						 ## Reason\n\n{}\n\n\
						 ## Existing file contents\n\n\
						 <existing_file>\n{existing}\n</existing_file>\n\n\
						 Return the complete updated file contents. Preserve \
						 unrelated behavior. Do not use Markdown fences, \
						 shell commands, or explanatory text.",
							file.path.display(),
							file.reason,
						);

						let content = self.generate_text(prompt).await?;

						tokio::fs::write(&path, content.as_bytes())
							.await
							.with_context(|| format!("updating {}", file.path.display()))?;

						Ok("Updated file".to_string())
					}
					BuildOperation::Delete => {
						let metadata = tokio::fs::symlink_metadata(&path)
							.await
							.with_context(|| format!("checking {}", file.path.display()))?;

						if !metadata.is_file() {
							bail!(
								"Delete target is not a regular file: {}",
								file.path.display()
							);
						}

						tokio::fs::remove_file(&path)
							.await
							.with_context(|| format!("deleting {}", file.path.display()))?;

						Ok("Deleted file".to_string())
					}
				}
			}
			.await;

			match result {
				Ok(message) => report.files.push(BuildFileResult {
					path: file.path.clone(),
					operation: file.operation.clone(),
					success: true,
					message,
				}),
				Err(error) => {
					report.files.push(BuildFileResult {
						path: file.path.clone(),
						operation: file.operation.clone(),
						success: false,
						message: format!("{error:#}"),
					});

					return Err(error).with_context(|| format!("Generate failed at {}", file.path.display()));
				}
			}
		}

		Ok(report)
	}

	pub async fn verify(
		&self,
		manifest: &BuildManifest,
		execution: &BuildExecutionReport,
		intent: &str,
		spec: &str,
		plan: &str,
	) -> Result<BuildVerification> {
		log::emit(E::ModelVerify);
		Self::validate_build_manifest(manifest, &self.workspace.cwd)?;

		let mut findings = Vec::new();

		for file in &manifest.files {
			let path = Self::resolve_build_path(&self.workspace.cwd, &file.path)?;

			let result = execution
				.files
				.iter()
				.find(|result| result.path == file.path && result.operation == file.operation);

			match result {
				Some(result) if result.success => {}
				Some(result) => {
					findings.push(format!(
						"{}: operation failed: {}",
						file.path.display(),
						result.message,
					));
					continue;
				}
				None => {
					findings.push(format!("{}: missing execution record", file.path.display(),));
					continue;
				}
			}

			match file.operation {
				BuildOperation::Create | BuildOperation::Update => match tokio::fs::metadata(&path).await {
					Ok(metadata) if metadata.is_file() => {}
					_ => findings.push(format!(
						"{}: expected file is missing or is not a regular file",
						file.path.display(),
					)),
				},
				BuildOperation::Delete => {
					if tokio::fs::symlink_metadata(&path).await.is_ok() {
						findings.push(format!(
							"{}: still exists after deletion",
							file.path.display(),
						));
					}
				}
			}
		}

		if !findings.is_empty() {
			return Ok(BuildVerification {
				passed: false,
				summary: "Build file-operation checks failed".into(),
				findings,
			});
		}

		let mut contents = String::new();

		for file in &manifest.files {
			if matches!(file.operation, BuildOperation::Delete) {
				continue;
			}

			let path = Self::resolve_build_path(&self.workspace.cwd, &file.path)?;
			let content = tokio::fs::read_to_string(&path)
				.await
				.with_context(|| format!("reading {}", file.path.display()))?;

			contents.push_str(&format!(
				"\n\n## {}\n\n<file_contents>\n{}\n</file_contents>",
				file.path.display(),
				content,
			));
		}

		let prompt = format!(
			"{PROMPT_BUILD_VERIFY}\n\n\
		 ## Intent\n\n{intent}\n\n\
		 ## Specification\n\n{spec}\n\n\
		 ## Plan\n\n{plan}\n\n\
		 ## Manifest\n\n{}\n\n\
		 ## Execution report\n\n{}\n\n\
		 ## Actual resulting file contents\n\n{contents}\n\n\
		 Return only JSON in this format:\n\
		 {{\"passed\":true,\"summary\":\"...\",\"findings\":[]}}\n\
		 Set passed=false for unmet requirements or implementation defects. \
		 Do not claim tests were run unless actual test results were supplied. \
		 No Markdown fences.",
			serde_json::to_string_pretty(manifest)?,
			serde_json::to_string_pretty(execution)?,
		);

		let response = self.generate_text(prompt).await?;

		let verification: BuildVerification = serde_json::from_str(response.trim())
			.context("parsing Verify response as BuildVerification")?;

		Ok(verification)
	}

	async fn generate_text(&self, prompt: String) -> Result<String> {
		let ctx = AgentCtx::with_workspace(prompt, self.workspace.clone());
		let response = prompt_chat(&ctx)
			.await
			.context("generating model response")?;
		Ok(response)
	}

	fn validate_build_manifest(manifest: &BuildManifest, workspace: &Path) -> Result<()> {
		if manifest.version != 1 {
			bail!("Unsupported Build manifest version: {}", manifest.version);
		}

		if manifest.files.len() > 500 {
			bail!("Build manifest exceeds the 500-file limit");
		}

		let mut seen = std::collections::HashSet::new();

		for file in &manifest.files {
			if file.reason.trim().is_empty() {
				bail!("Missing reason for {}", file.path.display());
			}

			if !seen.insert(file.path.clone()) {
				bail!("Duplicate manifest path: {}", file.path.display());
			}

			let path = Self::resolve_build_path(workspace, &file.path)?;

			match file.operation {
				BuildOperation::Create => {
					if path.exists() {
						bail!("Create target already exists: {}", file.path.display());
					}
				}
				BuildOperation::Update | BuildOperation::Delete => {
					let metadata = std::fs::symlink_metadata(&path)
						.with_context(|| format!("Target does not exist: {}", file.path.display()))?;

					if !metadata.is_file() {
						bail!("Target is not a regular file: {}", file.path.display());
					}
				}
			}
		}

		Ok(())
	}

	fn resolve_build_path(workspace: &Path, relative: &Path) -> Result<PathBuf> {
		if relative.as_os_str().is_empty() || relative.is_absolute() {
			bail!(
				"Build paths must be non-empty relative paths: {}",
				relative.display()
			);
		}

		for component in relative.components() {
			match component {
				Component::Normal(_) => {}
				_ => bail!("Invalid Build path: {}", relative.display()),
			}
		}

		let root = workspace
			.canonicalize()
			.with_context(|| format!("resolving workspace {}", workspace.display()))?;

		let target = root.join(relative);

		// Resolve the target if it exists. Otherwise resolve its nearest
		// existing ancestor, which also catches symlinks escaping the workspace.
		let mut ancestor = target.as_path();

		while !ancestor.exists() {
			ancestor = ancestor
				.parent()
				.ok_or_else(|| anyhow!("Cannot resolve {}", relative.display()))?;
		}

		let resolved_ancestor = ancestor
			.canonicalize()
			.with_context(|| format!("resolving path ancestor {}", ancestor.display()))?;

		if !resolved_ancestor.starts_with(&root) {
			bail!("Build path escapes workspace: {}", relative.display());
		}

		Ok(target)
	}
}

impl RuntimeMetrics {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn record_command(&mut self, duration_ms: u64) {
		self.commands_run += 1;
		self.command_durations_ms.push(duration_ms);
	}

	pub fn record_tool_call(&mut self, duration_ms: u64) {
		self.tool_calls += 1;
		self.tool_durations_ms.push(duration_ms);
	}

	pub fn record_stage(&mut self, duration_ms: u64) {
		self.stage_durations_ms.push(duration_ms);
	}

	pub fn record_test(&mut self, passed: bool) {
		self.tests_run += 1;
		if passed {
			self.tests_passed += 1;
		} else {
			self.tests_failed += 1;
			self.test_failures += 1;
		}
	}

	pub fn record_compile_failure(&mut self) {
		self.compile_failures += 1;
		self.failures += 1;
	}

	pub fn record_failure(&mut self) {
		self.failures += 1;
	}

	pub fn record_timeout(&mut self) {
		self.timeouts += 1;
	}

	pub fn record_retry(&mut self) {
		self.retries += 1;
	}

	pub fn record_attempt(&mut self) {
		self.attempts += 1;
	}

	pub fn record_recovery(&mut self) {
		self.recovery_count += 1;
	}

	pub fn record_discovery(&mut self, rules_evaluated: u64, rules_matched: u64) {
		self.discovery_runs += 1;
		self.rules_evaluated += rules_evaluated;
		self.rules_matched += rules_matched;
	}

	pub fn record_tokens(&mut self, input: u64, output: u64) {
		self.tokens_input += input;
		self.tokens_output += output;
	}

	pub fn record_file_read(&mut self) {
		self.files_read += 1;
	}

	pub fn record_file_write(&mut self) {
		self.files_written += 1;
	}

	pub fn record_file_create(&mut self) {
		self.files_created += 1;
		self.files_written += 1;
	}

	pub fn record_file_delete(&mut self) {
		self.files_deleted += 1;
	}

	// ── Derived metrics ──────────────────────────────────────────

	pub fn tokens_per_attempt(&self) -> u64 {
		if self.attempts == 0 {
			return 0;
		}

		(self.tokens_input + self.tokens_output) / self.attempts
	}

	pub fn files_per_attempt(&self) -> u64 {
		if self.attempts == 0 {
			return 0;
		}

		(self.files_read + self.files_written) / self.attempts
	}

	pub fn tool_calls_per_attempt(&self) -> u64 {
		if self.attempts == 0 {
			return 0;
		}

		self.tool_calls / self.attempts
	}

	pub fn test_pass_rate(&self) -> f64 {
		if self.tests_run == 0 {
			return 0.0;
		}

		self.tests_passed as f64 / self.tests_run as f64
	}

	pub fn rule_match_rate(&self) -> f64 {
		if self.rules_evaluated == 0 {
			return 0.0;
		}

		self.rules_matched as f64 / self.rules_evaluated as f64
	}
}

#[derive(Clone, Debug, Default)]
pub struct AgentRegistry;
#[derive(Clone, Debug)]
pub struct AgentRuntime {
	pub event_tx: UnboundedSender<RuntimeEvent>,
	pub registry: AgentRegistry,
	pub workspace: CtxWorkspace,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeMetrics {
	// ── Interaction ──────────────────────────────────────────────
	pub prompts_sent: u64,
	pub responses_received: u64,

	// ── Files ────────────────────────────────────────────────────
	pub files_read: u64,
	pub files_written: u64,
	pub files_created: u64,
	pub files_deleted: u64,

	// ── Execution ────────────────────────────────────────────────
	pub commands_run: u64,
	pub tests_run: u64,
	pub tests_passed: u64,
	pub tests_failed: u64,
	pub tool_calls: u64,

	// ── Attempts / reliability ──────────────────────────────────
	pub attempts: u64,
	pub retries: u64,
	pub failures: u64,
	pub timeouts: u64,
	pub cancellations: u64,

	pub stuck_count: u64,
	pub recovery_count: u64,

	pub invalid_actions: u64,
	pub rejected_actions: u64,

	// ── Build / test failures ────────────────────────────────────
	pub compile_failures: u64,
	pub test_failures: u64,

	// ── Tokens / context ─────────────────────────────────────────
	pub tokens_input: u64,
	pub tokens_output: u64,

	pub context_tokens: u64,
	pub disclosed_tokens: u64,
	pub prompt_tokens: u64,
	pub history_tokens: u64,
	pub workspace_tokens: u64,

	// ── Discovery ────────────────────────────────────────────────
	pub discovery_runs: u64,
	pub rules_evaluated: u64,
	pub rules_matched: u64,

	// ── Timing ───────────────────────────────────────────────────
	pub elapsed_ms: u64,
	pub command_durations_ms: Vec<u64>,
	pub tool_durations_ms: Vec<u64>,
	pub stage_durations_ms: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildManifest {
	pub version: u32,
	pub files: Vec<BuildFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFile {
	pub path: PathBuf,
	pub operation: BuildOperation,
	pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuildOperation {
	Create,
	Update,
	Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildVerification {
	pub passed: bool,
	pub summary: String,
	pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildExecutionReport {
	pub files: Vec<BuildFileResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildFileResult {
	pub path: PathBuf,
	pub operation: BuildOperation,
	pub success: bool,
	pub message: String,
}
