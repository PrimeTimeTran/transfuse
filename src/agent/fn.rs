use super::*;
use crate::prelude::{DEFAULT_MODEL, *};
use anyhow::Context as CtxAnyhow;

const DEFAULT_CTX_WINDOW_SIZE: u32 = 16_384;

pub fn ctx_window_size() -> u32 {
	std::env::var("CTX_WINDOW_SIZE")
		.ok()
		.and_then(|value| value.parse().ok())
		.unwrap_or(DEFAULT_CTX_WINDOW_SIZE)
}

pub fn build_prompt(run: &AgentRun) -> String {
	let ctx = &run.ctx;
	let workspace = fmt_workspace(&ctx.workspace);
	let history = fmt_history(&ctx.history, 3, 3);
	let execution_ledger = fmt_execution(&run.commands, 5, 5, 4_000);
	let task = ctx.prompt.as_deref().unwrap_or("");
	let recovery_error = run
		.errors
		.iter()
		.rev()
		.find(|error| error.kind == "completion_validation")
		.map(|error| error.message.as_str());
	let expected_outputs = ctx
		.task
		.as_ref()
		.and_then(|task| serde_json::to_string_pretty(&task.expected_outputs).ok())
		.unwrap_or_else(|| "[]".into());
	let prompt = if let Some(validation_errors) = recovery_error {
		let recovery_template = ctx
			.task
			.as_ref()
			.and_then(|task| task.recovery_prompt.as_deref())
			.unwrap_or(DEFAULT_RECOVERY_PROMPT);
		recovery_template
			.replace("{task}", task)
			.replace("{workspace}", &workspace)
			.replace("{todo}", &ctx.todo.join("\n"))
			.replace("{validation_errors}", validation_errors)
			.replace(
				"{execution_ledger}",
				&fmt_execution(&run.commands, 3, 3, 2_000),
			)
			.replace("{history}", &fmt_history(&ctx.history, 2, 3))
			.replace("{expected_outputs}", &expected_outputs)
	} else {
		ACTION_PROMPT_EXECUTION
			.replace("{task}", task)
			.replace("{workspace}", &workspace)
			.replace("{history}", &history)
			.replace("{execution_ledger}", &execution_ledger)
			.replace("{expected_outputs}", &expected_outputs)
	};

	let mut report = format!(
		"{}\n{}\n{}\n",
		fmt_kv("BUILD_PROMPT:", &format!("{:?}", truncate(task, 50))),
		fmt_kv("CWD:", &ctx.workspace.cwd.display().to_string()),
		fmt_kv("EXPECTED OUTPUTS:", &expected_outputs),
	);
	report.push_str(&fmt_section("WORKSPACE"));
	report.push_str(&workspace);
	report.push_str(&fmt_section("HISTORY"));
	report.push_str(&history);

	report.push_str(&fmt_section("EXECUTION LEDGER"));
	report.push_str(&execution_ledger);

	report.push_str(&format!(
		"\n{}\n",
		fmt_kv("FINAL PROMPT CHARS:", &prompt.len().to_string()),
	));
	report.push_str(&format!(
		"{}\n",
		fmt_kv("FINAL PROMPT LINES:", &prompt.lines().count().to_string()),
	));

	section!(&report);

	prompt
}
pub fn build_prompt_from_ctx(ctx: &AgentCtx) -> String {
	let workspace = fmt_workspace(&ctx.workspace);
	let history = fmt_history(&ctx.history, 3, 3);
	let prompt = build_sys_action(
		ACTION_PROMPT_EXECUTION,
		&[&ctx.prompt.as_deref().unwrap_or(""), &workspace, &history],
	);
	section!("build_prompt_from_ctx");
	println!("ctx.history ({} entries):", ctx.history.len());
	println!(
		"prompt ({} lines, {} chars):\n{}",
		prompt.lines().count(),
		prompt.len(),
		preview_lines(&prompt, PROMPT_PREVIEW_LINES)
	);
	prompt
}
pub fn build_sys_action(template: &str, args: &[&str]) -> String {
	let placeholders = ["{task}", "{workspace}", "{history}"];

	debug_assert_eq!(
		placeholders.len(),
		args.len(),
		"build_sys_action: placeholder count doesn't match argument count",
	);
	let mut prompt = template.to_string();
	for (placeholder, arg) in placeholders.iter().zip(args) {
		prompt = prompt.replace(placeholder, arg);
	}
	prompt
}
pub fn build_sys_prompt(template: &str, prompt: &str) -> String {
	template.replace("{}", prompt)
}

pub fn language_from_extension(extension: &str) -> Option<String> {
	let language = match extension {
		"rs" => "Rust",
		"js" => "JavaScript",
		"jsx" => "JavaScript",
		"ts" => "TypeScript",
		"tsx" => "TypeScript",
		"py" => "Python",
		"go" => "Go",
		"java" => "Java",
		"c" => "C",
		"h" => "C",
		"cpp" => "C++",
		"cc" => "C++",
		"cxx" => "C++",
		"hpp" => "C++",
		"cs" => "C#",
		"swift" => "Swift",
		"kt" => "Kotlin",
		"kts" => "Kotlin",
		"rb" => "Ruby",
		"php" => "PHP",
		"sh" => "Shell",
		"bash" => "Shell",
		"zsh" => "Shell",
		"fish" => "Shell",
		"html" => "HTML",
		"css" => "CSS",
		"scss" => "SCSS",
		"json" => "JSON",
		"toml" => "TOML",
		"yaml" => "YAML",
		"yml" => "YAML",
		"xml" => "XML",
		"md" => "Markdown",
		"sql" => "SQL",
		_ => return None,
	};
	Some(language.to_string())
}

pub fn parse_git_status_paths(status: &str) -> Vec<PathBuf> {
	let mut paths = Vec::new();

	for line in status.lines() {
		// `git status --short` starts with two status columns.
		if line.len() < 4 {
			continue;
		}

		let status_code = &line[..2];
		let path = line[3..].trim();

		if path.is_empty() || status_code == "!!" {
			continue;
		}

		// For renames, porcelain output is generally "old -> new".
		let path = path
			.rsplit_once(" -> ")
			.map(|(_, new_path)| new_path)
			.unwrap_or(path)
			.trim_matches('"');

		paths.push(PathBuf::from(path));
	}

	paths.sort();
	paths.dedup();
	paths
}
pub fn preview_lines(text: &str, max_lines: usize) -> String {
	let lines: Vec<&str> = text.lines().collect();
	let (lines, truncated) = if lines.len() <= max_lines {
		(lines.as_slice(), 0)
	} else {
		(&lines[..max_lines], lines.len() - max_lines)
	};
	let mut output = String::new();
	for line in lines {
		output.push_str("\x1b[2m  ");
		output.push_str(line);
		output.push_str("\x1b[0m\n");
	}
	if truncated > 0 {
		output.push_str("\n");
		output.push_str(&format!(
			"\x1b[2m  ... {} more lines truncated\x1b[0m\n",
			truncated
		));
	}
	output
}
pub fn preview(value: impl std::fmt::Debug, max_len: usize) -> String {
	let value = format!("{value:?}");
	if value.len() > max_len {
		format!(
			"{}... [truncated, true_len={}]",
			&value[..max_len],
			value.len()
		)
	} else {
		value
	}
}

pub fn structured_prompt_chat(ctx: &AgentCtx) -> String {
	format!(
		r#"
			You are a helpful assistant.
			User request:
			{}
			History:
			{}
			Respond normally. No JSON. Just text.
		"#,
		&ctx.prompt.as_deref().unwrap_or(""),
		fmt_history(&ctx.history, 3, 3)
	)
}
pub fn validate_agent_completion(
	workspace_root: &Path,
	expected_outputs: &[PathBuf],
) -> Result<AgentCompletionValidation> {
	let mut errors = Vec::new();
	let workspace_root = workspace_root.canonicalize().with_context(|| {
		format!(
			"canonicalizing agent workspace {}",
			workspace_root.display()
		)
	})?;
	for required_file in expected_outputs {
		if required_file.is_absolute()
			|| required_file
				.components()
				.any(|component| matches!(component, std::path::Component::ParentDir))
		{
			errors.push(format!(
				"Output path escapes workspace: {}",
				required_file.display()
			));
			continue;
		}
		let path = workspace_root.join(required_file);
		match path.canonicalize() {
			Ok(path) if path.starts_with(&workspace_root) && path.is_file() => {
				match std::fs::metadata(&path) {
					Ok(metadata) if metadata.len() > 0 => {}
					Ok(_) => errors.push(format!(
						"Expected output is empty: {}",
						required_file.display()
					)),
					Err(error) => errors.push(format!(
						"Could not inspect {}: {error}",
						required_file.display()
					)),
				}
			}
			Ok(_) => errors.push(format!(
				"Expected output is not a workspace file: {}",
				required_file.display()
			)),
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
				errors.push(format!(
					"Expected output is missing: {}",
					required_file.display()
				));
			}
			Err(error) => errors.push(format!(
				"Could not inspect {}: {error}",
				required_file.display()
			)),
		}
	}
	Ok(AgentCompletionValidation { errors })
}

const DEFAULT_RECOVERY_PROMPT: &str = r#"
You are continuing an implementation after runtime verification rejected the previous finish.
Read the supplied task and Plan, inspect the current workspace, then address the reported failures directly.
Choose one concrete action at a time. Do not claim completion or choose finish until the failed checks are corrected and inspected.

## Task and Plan
{task}

## Current workspace
{workspace}

## Work remaining
{todo}

## Runtime verification failures
{validation_errors}

## Recent execution evidence
{execution_ledger}

## Recent history
{history}

Return exactly one supported action JSON object.
"#;

#[cfg(test)]
mod agent_recovery_tests {
	use super::*;

	#[test]
	fn completion_rejects_missing_and_empty_outputs() {
		let workspace = std::env::temp_dir().join(format!("agent-test-{}", uuid::Uuid::new_v4()));
		std::fs::create_dir_all(&workspace).unwrap();
		let outputs = vec![PathBuf::from("hello-world.cjs")];

		let missing = validate_agent_completion(&workspace, &outputs).unwrap();
		assert!(!missing.is_valid());
		assert!(missing.errors.iter().any(|error| error.contains("missing")));

		std::fs::write(workspace.join("hello-world.cjs"), "").unwrap();
		let empty = validate_agent_completion(&workspace, &outputs).unwrap();
		assert!(empty.errors.iter().any(|error| error.contains("empty")));

		std::fs::remove_dir_all(workspace).unwrap();
	}

	#[test]
	fn rejected_finish_uses_compact_recovery_prompt() {
		let task = AgentTask::new("implement the Plan".into())
			.with_expected_outputs(vec![PathBuf::from("hello-world.cjs")])
			.with_recovery_prompt("Recovery: {validation_errors}\nTODO: {todo}\nTask: {task}");
		let mut context = AgentCtx::with_workspace(task.prompt.clone(), CtxWorkspace::default());
		context.task = Some(task);
		context.todo = vec!["Write hello-world.cjs".into()];
		let mut run = AgentRun::new(context);
		run.record_error(AgentError {
			step: Some(1),
			kind: "completion_validation".into(),
			message: "Expected output is empty: hello-world.cjs".into(),
			raw_response: None,
			recoverable: true,
			timestamp: chrono::Utc::now(),
		});

		let prompt = build_prompt(&run);
		assert!(prompt.contains("Recovery: Expected output is empty"));
		assert!(prompt.contains("TODO: Write hello-world.cjs"));
		assert!(!prompt.contains("PLAN-FIRST EXECUTION PROTOCOL"));
	}
}

pub fn validate_shell_command(command: &str) -> anyhow::Result<()> {
	use anyhow::bail;
	if command.trim().is_empty() {
		bail!("Command cannot be empty.");
	}
	if command.len() > 2_000 {
		bail!("Command exceeds the 2000-character limit.");
	}
	Ok(())
}

pub fn reconcile_agent_artifacts(
	ctx: &mut AgentCtx,
	before: &WSSnapshot,
	after: &WSSnapshot,
) -> Result<Vec<PathBuf>> {
	let changes = before.diff(after);
	let changed_paths = parse_git_status_paths(&changes.git_status);
	for path in &changed_paths {
		let absolute_path = if path.is_absolute() {
			path.clone()
		} else {
			ctx.workspace.cwd.join(path)
		};
		let path_string = absolute_path.to_string_lossy().into_owned();
		if absolute_path.is_file() {
			ctx.workspace.add_file(absolute_path.clone())?;
			if !ctx
				.artifacts
				.iter()
				.any(|existing| matches!(existing, Artifact::FileWrite { path } if path == &path_string))
			{
				ctx
					.artifacts
					.push(Artifact::FileWrite { path: path_string });
			}
		} else {
			ctx
				.workspace
				.files
				.retain(|file| Path::new(&file.path) != absolute_path);
			ctx.artifacts.retain(
				|artifact| !matches!(artifact, Artifact::FileWrite { path } if path == &path_string),
			);
		}
	}
	Ok(changed_paths)
}

pub fn touched_required_output(command: &str, expected_outputs: &[PathBuf]) -> Option<PathBuf> {
	let tokens = command
		.split_whitespace()
		.map(|token| token.trim_matches(['\'', '"']))
		.collect::<Vec<_>>();
	let touch_index = tokens
		.iter()
		.position(|token| token.rsplit('/').next() == Some("touch"))?;
	let arguments = tokens
		.iter()
		.skip(touch_index + 1)
		.take_while(|token| !matches!(**token, "&&" | "||" | ";" | "|"))
		.filter(|token| !token.starts_with('-'))
		.collect::<Vec<_>>();
	expected_outputs
		.iter()
		.find(|required| {
			arguments
				.iter()
				.any(|argument| Path::new(argument).ends_with(*required))
		})
		.cloned()
}
pub fn shell_quote(value: &str) -> String {
	if value.is_empty() {
		return "''".to_owned();
	}
	if value
		.chars()
		.all(|c| c.is_ascii_alphanumeric() || "_./:-=+".contains(c))
	{
		return value.to_owned();
	}
	format!("'{}'", value.replace('\'', "'\\''"))
}

pub async fn prompt_chat(ctx: &AgentCtx) -> Result<String> {
	let prompt = structured_prompt_chat(ctx);
	let result = ollama_generate(&prompt, None, false).await?;
	Ok(result)
}
pub async fn prompt_ollama_json<T>(prompt: &str) -> Result<T>
where
	T: DeserializeOwned,
{
	let raw = ollama_generate(prompt, Some("You are a helpful assistant"), true)
		.await
		.context("Ollama generation failed")?;
	serde_json::from_str(&raw).with_context(|| {
		format!(
			"Failed to deserialize Ollama JSON response ({} bytes): {:?}",
			raw.len(),
			raw.chars().take(2_000).collect::<String>(),
		)
	})
}
pub async fn build_action(prompt: &str) -> Result<LlmAction> {
	use std::time::Instant;

	let started = Instant::now();

	section_info!(
		"build_action",
		metrics: [
			"model" => DEFAULT_MODEL,
			"elapsed" => format!("{:.1?}", started.elapsed()),
			"prompt" => format!("{} chars", prompt.len()),
			"system" => format!("{} chars", JSON_PROMPT_EXECUTION.len()),
		],
	);

	let payload = serde_json::json!({
		"model": DEFAULT_MODEL,
		"system": JSON_PROMPT_EXECUTION,
		"prompt": prompt,
		"stream": false,
		"options": {
			"num_ctx": ctx_window_size(),
		},
		"format": "json"
	});

	let client = reqwest::Client::builder()
		.connect_timeout(std::time::Duration::from_secs(5))
		.timeout(std::time::Duration::from_secs(120))
		.build()
		.context("building HTTP client")?;
	let request_started = std::time::Instant::now();

	let res = client
		.post(AGENT_GEN_URL)
		.json(&payload)
		.send()
		.await
		.with_context(|| {
			format!(
				"sending model request to {} after {:?}",
				AGENT_GEN_URL,
				request_started.elapsed()
			)
		})?;
	let status = res.status();

	let body_started = Instant::now();

	let body = res
		.text()
		.await
		.context("reading model HTTP response body")?;

	let body_elapsed = body_started.elapsed();
	if !status.is_success() {
		eprintln!(
			"{}",
			fmt_kv("HTTP ERROR BODY:", &truncate_output(&body, 2_000))
		);

		return Err(anyhow::anyhow!("Model endpoint returned HTTP {status}"));
	}

	let res: serde_json::Value =
		serde_json::from_str(&body).context("parsing model HTTP response JSON")?;
	let response = res["response"]
		.as_str()
		.context("Ollama response JSON is missing a string `response` field")?;
	let mut raw: LlmAction = serde_json::from_str(response).context("parsing model action JSON")?;
	if raw.command.is_none() {
		if let Some(args) = raw.args.as_ref() {
			if !args.is_empty() {
				raw.command = Some(
					args
						.iter()
						.map(|arg| shell_quote(arg))
						.collect::<Vec<_>>()
						.join(" "),
				);
			}
		}
	}
	log_build_action(
		request_started.elapsed(),
		status,
		body.len(),
		body.lines().count(),
		body_elapsed,
		response.len(),
		&raw,
		started.elapsed(),
	);
	Ok(raw)
}
pub async fn ollama_generate(prompt: &str, system: Option<&str>, json: bool) -> Result<String> {
	let client = reqwest::Client::new();

	let mut payload = serde_json::json!({
		"model": DEFAULT_MODEL,
		"prompt": prompt,
		"stream": false,
		"options": {
			"num_ctx": ctx_window_size(),
		},
	});

	if let Some(sys_msg) = system {
		payload["system"] = serde_json::json!(sys_msg);
	}

	if json {
		payload["format"] = serde_json::json!("json");
	}

	let response = client
		.post(AGENT_GEN_URL)
		.json(&payload)
		.send()
		.await
		.context("Failed to send Ollama request")?;

	let status = response.status();
	let body = response
		.text()
		.await
		.context("Failed to read Ollama response body")?;

	if !status.is_success() {
		anyhow::bail!(
			"Ollama returned HTTP {status}: {}",
			body.chars().take(2_000).collect::<String>()
		);
	}

	let res: serde_json::Value =
		serde_json::from_str(&body).context("Ollama returned invalid response JSON")?;

	if let Some(error) = res.get("error").and_then(|v| v.as_str()) {
		anyhow::bail!("Ollama error: {error}");
	}

	res
		.get("response")
		.and_then(serde_json::Value::as_str)
		.map(str::to_owned)
		.ok_or_else(|| {
			anyhow::anyhow!(
				"Ollama response missing string field `response`: {}",
				body.chars().take(2_000).collect::<String>()
			)
		})
}

fn log_build_action(
	request_elapsed: std::time::Duration,
	status: reqwest::StatusCode,
	body_len: usize,
	body_lines: usize,
	body_elapsed: std::time::Duration,
	response_len: usize,
	action: &LlmAction,
	total_elapsed: std::time::Duration,
) {
	section_info!(
		"build_action",
		metrics: [
			"status" => status,
			"response after" => format!("{:.3}s", request_elapsed.as_secs_f64()),
			"total" => format!("{:.3}s", total_elapsed.as_secs_f64()),
			"body chars" => body_len,
			"body lines" => body_lines,
			"read" => format!("{:.3}s", body_elapsed.as_secs_f64()),
			"response chars" => response_len,
		],
		"request" => "SENDING",
		"action JSON" => "PARSED",
		"action" => format!("{:?}", action.action),
		"command" => format!("{:?}", action.command),
		"message" => format!("{:?}", action.message),
		"return" => "OK",
	);
}
