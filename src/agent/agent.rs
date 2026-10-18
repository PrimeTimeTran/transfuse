use super::*;

fn log_run(run: &mut AgentRun, message: impl Into<String>) {
	let message = message.into();
	println!(
		"{} {}",
		colorize(BOLD, "[🤖 AGENT]"),
		colorize(DIM, &message),
	);
	run.record_log(format!("[🤖 AGENT] {message}"));
}
fn print_run_metrics(run: &mut AgentRun, files_seen: u128, max_steps: usize) {
	let prompt = run.ctx.prompt.as_deref().unwrap_or("");

	let prompt_chars = prompt.chars().count();
	let estimated_tokens = prompt_chars.div_ceil(6);

	section_info!(
		"🏗️ run_agent_loop 🔁",
		metrics: [
			"prompt lines" => prompt.lines().count(),
			"prompt chars" => prompt_chars,
			"prompt tokens ≈" => estimated_tokens,

			"max steps" => max_steps,
			"expected outputs" => run.ctx.task.as_ref().map_or(0, |task| task.expected_outputs.len()),
			"history" => run.ctx.history.len(),
			"commands" => run.commands.len(),
			"errors" => run.errors.len(),
			"logs" => run.ctx.logs.len(),
			"files" => files_seen,
			"preview lines" => PROMPT_PREVIEW_LINES,
		],
		"prompt preview" => preview_lines(prompt, PROMPT_PREVIEW_LINES),
	);

	run.record_log(format!(
		"Initial context: prompt_chars={prompt_chars}, estimated_tokens={estimated_tokens}, workspace={}",
		run.ctx.workspace,
	));
}
fn log_step(run: &mut AgentRun, step: usize, message: impl std::fmt::Display) {
	log_run(run, format!("STEP {step}: {message}"));
}
fn print_steps(
	run: &mut AgentRun,
	step: usize,
	max_steps: usize,
	tokens_seen: u128,
	files_seen: u128,
) {
	log_run(
		run,
		format!(
			"STEP {step}/{max_steps}: history={}, commands={}, errors={}, logs={}, tokens={}, files={}",
			run.ctx.history.len(),
			run.commands.len(),
			run.errors.len(),
			run.ctx.logs.len(),
			tokens_seen,
			files_seen,
		),
	);
}

impl Agent {
	fn emit_thinking(&self, task: &AgentTask, event_tx: &UnboundedSender<RuntimeEvent>) {
		let mut t = task.clone();
		t.prompt = crate::agent::log::truncate(&task.prompt, 50);
		let log_event = RuntimeEvent::Agent(AgentEvent::Thinking { task: t });
		if let Err(error) = log::event(&log_event) {
			eprintln!("Failed to write agent event log: {error:#}");
		}
		let event = RuntimeEvent::Agent(AgentEvent::Thinking { task: task.clone() });
		if let Err(error) = event_tx.send(event) {
			eprintln!("Failed to emit thinking event: {error}");
		}
	}
	fn emit_finished(&self, event_tx: &UnboundedSender<RuntimeEvent>, result: TaskResult) {
		let event = RuntimeEvent::Agent(AgentEvent::Finished { result });

		if let Err(error) = log::event(&event) {
			eprintln!("Failed to write agent event log: {error:#}");
		}

		if let Err(error) = event_tx.send(event) {
			eprintln!("Failed to emit finished event: {error}");
		}
	}
	fn fail_step_limit(
		&self,
		task_id: uuid::Uuid,
		mut run: AgentRun,
		max_steps: usize,
	) -> TaskResult {
		let message = format!(
			"Agent exceeded {max_steps} reasoning steps; commands={}, errors={}.",
			run.commands.len(),
			run.errors.len(),
		);

		run.record_error(AgentError {
			step: Some(max_steps),
			kind: "step_limit".into(),
			message: message.clone(),
			raw_response: None,
			recoverable: false,
			timestamp: chrono::Utc::now(),
		});

		TaskResult::failed(
			task_id,
			run.ctx,
			"Step limit exceeded",
			Some(message),
			run.commands,
			run.errors,
		)
	}
	fn guard_action(
		&self,
		action: &AgentAction,
		run: &AgentRun,
		guard: &mut AgentGuard,
	) -> Result<GuardDecision> {
		let touch_command = match action {
			AgentAction::RunCommand { command }
				if !command.contains("&&")
					&& !command.contains(';')
					&& !command.contains('|')
					&& !command.contains('\n') =>
			{
				Some(command.clone())
			}
			AgentAction::RunProgram { program, args, .. }
				if program.rsplit('/').next() == Some("touch") =>
			{
				Some(
					std::iter::once(program.clone())
						.chain(args.iter().cloned())
						.collect::<Vec<_>>()
						.join(" "),
				)
			}
			_ => None,
		};

		if let (Some(command), Some(task)) = (touch_command, run.ctx.task.as_ref()) {
			if let Some(path) = touched_required_output(&command, &task.expected_outputs) {
				return Ok(GuardDecision::Reject(format!(
					"`touch` only creates an empty placeholder for required output `{}`. \
				Write the implementation contents to that file; do not create placeholder files.",
					path.display(),
				)));
			}
		}

		let command = match action {
			AgentAction::RunCommand { command } => Some(command.as_str()),
			AgentAction::RunProgram { program, args, .. } => match program.as_str() {
				"sh" | "bash" | "zsh" => args
					.iter()
					.position(|arg| arg == "-c")
					.and_then(|index| args.get(index + 1))
					.map(String::as_str),
				_ => None,
			},

			_ => None,
		};
		let Some(command) = command else {
			return Ok(GuardDecision::Allow);
		};
		let command = command.trim();
		if command.is_empty() {
			return Ok(GuardDecision::Reject("Command cannot be empty.".into()));
		}
		if command.len() > 10_000 {
			return Ok(GuardDecision::Reject(
				"Command exceeds the 10000-character limit. \
				Split unusually large commands into smaller operations."
					.into(),
			));
		}
		let count = guard
			.recent_commands
			.entry(command.to_string())
			.or_insert(0);
		*count += 1;
		if *count > 2 {
			return Ok(GuardDecision::Reject(
				"This exact command has already been proposed multiple times. \
				Inspect prior results and change your approach."
					.into(),
			));
		}

		// Compare against executed commands. Permit one retry, since a
		// command may legitimately need to be rerun after a transient failure.
		let previous_runs = run
			.commands
			.iter()
			.filter(|record| record.command.trim() == command)
			.count();

		if previous_runs >= 2 {
			return Ok(GuardDecision::Reject(
				"This exact command has already been executed twice. \
				Inspect its recorded output before attempting another approach."
					.into(),
			));
		}

		Ok(GuardDecision::Allow)
	}

	fn handle_context(
		&self,
		task: &AgentTask,
		event_tx: &UnboundedSender<RuntimeEvent>,
		run: &mut AgentRun,
		path: Option<String>,
	) {
		let message = match path {
			Some(path) => format!(
				"Context requested for path: {path}. \
			Inspect it using a supported command, respecting the task's scope."
			),
			None => format!(
				"Context requested. Current history entries: {}. \
			Use supported commands to inspect only what the task authorizes.",
				run.ctx.history.len(),
			),
		};
		let msg = crate::agent::log::truncate(&message, 50);
		let event = RuntimeEvent::Agent(AgentEvent::Working {
			task: task.clone(),
			message: msg.clone(),
		});

		if let Err(error) = log::event(&event) {
			eprintln!("Failed to write agent event log: {error:#}");
		}

		if let Err(error) = event_tx.send(event) {
			eprintln!("Failed to emit context event: {error}");
		}

		run.ctx.history.push(AgentObservation::Current { message });
	}
	fn handle_current(
		&self,
		task: &AgentTask,
		event_tx: &UnboundedSender<RuntimeEvent>,
		run: &mut AgentRun,
		message: String,
	) {
		let now = chrono::Local::now().format("%Y-%m-%d").to_string();
		let message = format!("Context update: The current date is {now}. {message}");

		let event = RuntimeEvent::Agent(AgentEvent::Working {
			task: task.clone(),
			message: message.clone(),
		});

		if let Err(error) = log::event(&event) {
			eprintln!("Failed to write agent event log: {error:#}");
		}

		if let Err(error) = event_tx.send(event) {
			eprintln!("Failed to emit current event: {error}");
		}

		run.ctx.history.push(AgentObservation::Current { message });
	}
	fn reject_action(&self, run: &mut AgentRun, step: usize, reason: String) {
		eprintln!("Rejecting agent action at step {step}: {reason}");
		run.record_error(AgentError {
			step: Some(step),
			kind: "action_guard".into(),
			message: reason.clone(),
			raw_response: None,
			recoverable: true,
			timestamp: chrono::Utc::now(),
		});

		run.ctx.history.push(AgentObservation::Current {
			message: format!(
				"ACTION REJECTED BEFORE EXECUTION\n\
			Reason: {reason}\n\n\
			No command was executed for this action. \
			Choose a different bounded action. Do not repeat \
			the rejected action unchanged."
			),
		});
	}
	fn try_finish(
		&self,
		task_id: uuid::Uuid,
		run: &mut AgentRun,
		baseline: &WSSnapshot,
		step: usize,
		message: String,
	) -> Result<Option<TaskResult>> {
		let workspace_final = WSSnapshot::capture(&self.workspace.cwd)?;
		let final_changed_paths = reconcile_agent_artifacts(&mut run.ctx, baseline, &workspace_final)?;

		let expected_outputs = run
			.ctx
			.task
			.as_ref()
			.map_or(&[][..], |task| task.expected_outputs.as_slice());
		let validation = validate_agent_completion(&self.workspace.cwd, expected_outputs)?;

		if !validation.is_valid() {
			run.ctx.todo = validation.errors.clone();
			let feedback = format!(
				"COMPLETION REJECTED: runtime validation failed.\n\
				You claimed the task was complete, but these checks failed:\n{}\n\n\
				Correct the actual files, then inspect and verify them. \
				Do not simply repeat the completion claim.",
				validation.errors().join("\n"),
			);

			eprintln!("{feedback}");

			run.record_error(AgentError {
				step: Some(step),
				kind: "completion_validation".into(),
				message: feedback.clone(),
				raw_response: None,
				recoverable: true,
				timestamp: chrono::Utc::now(),
			});

			run
				.ctx
				.history
				.push(AgentObservation::Current { message: feedback });

			return Ok(None);
		}

		let final_message = format!(
			"{message}\n\nRuntime-observed changed files:\n{}",
			fmt_changed_files(&final_changed_paths),
		);

		Ok(Some(TaskResult::completed_with_summary(
			task_id,
			std::mem::replace(
				&mut run.ctx,
				AgentCtx::with_workspace(String::new(), (*self.workspace).clone()),
			),
			final_message,
			run.commands.clone(),
			run.errors.clone(),
		)))
	}

	async fn pick_mode(&self, ctx: &AgentCtx) -> Result<AgentMode> {
		let prompt = build_sys_prompt(DECIDE_PROMPT, &ctx.prompt.as_deref().unwrap_or(""));
		let raw: LlmMode = prompt_ollama_json(&prompt).await?;

		Ok(match raw.mode.as_str() {
			// "chat" => AgentMode::Chat,
			_ => AgentMode::Tool,
		})
	}
	async fn pick_action(&self, run: &AgentRun) -> Result<AgentAction> {
		// eprintln!("{}", fmt_kv("pick_action:", "ENTER"));
		let prompt = build_prompt(run);

		// eprintln!("ACTION PROMPT CHARS: {}", prompt.len());
		// eprintln!(
		// 	"ACTION PROMPT CONTAINS PLAN: {}",
		// 	prompt.contains("# Plan:")
		// );
		// eprintln!("{}", fmt_kv("pick_action:", "CALLING build_action"));
		let raw = build_action(&prompt).await?;
		// eprintln!("{}", fmt_kv("pick_action:", "build_action RETURNED"));
		let action = AgentAction::try_from(raw)?;
		// eprintln!("{}", fmt_kv("pick_action:", &format!("PARSED {action:?}")));
		Ok(action)
	}
	async fn select_action(&self, run: &mut AgentRun, step: usize) -> Result<Option<AgentAction>> {
		const MAX_SELECTION_ERRORS: usize = 3;
		match self.pick_action(&run).await {
			Ok(action) => {
				// A valid response breaks the consecutive failure streak.
				Ok(Some(action))
			}
			Err(error) => {
				let message = format!("Action selection failed: {error:#}");
				tracing::error!(message);

				let previous_failures = run
					.errors
					.iter()
					.rev()
					.take_while(|error| error.kind == "action_selection")
					.count();

				let consecutive_failures = previous_failures + 1;

				run.record_error(AgentError {
					step: Some(step),
					kind: "action_selection".into(),
					message: message.clone(),
					raw_response: None,
					recoverable: consecutive_failures < MAX_SELECTION_ERRORS,
					timestamp: chrono::Utc::now(),
				});

				if consecutive_failures >= MAX_SELECTION_ERRORS {
					return Ok(None);
				}

				run.ctx.history.push(AgentObservation::Current {
					message: format!(
						"INVALID ACTION RESPONSE\n\
					Reason: {message}\n\n\
					No command from this response was executed.\n\
					Return one valid JSON action using the supported schema. \
					For shell execution, use `run_command` with a non-empty \
					`command` field. For completion, use `finish` with a \
					`message` field. Do not repeat an invalid response."
					),
				});

				Ok(Some(AgentAction::Current {
					message: "Action selection failed; corrective feedback was added \
					to the history. Choose a valid next action."
						.into(),
				}))
			}
		}
	}
	async fn execute_shell_action(
		&self,
		run: &mut AgentRun,
		step: usize,
		command: String,
	) -> Result<()> {
		println!("COMMAND: {command}");

		let workspace_before = WSSnapshot::capture(&self.workspace.cwd)?;
		let mut shell_command = ShellCommand::shell(command.clone());
		shell_command.cwd = Some(self.workspace.cwd.clone());

		let result = match self.tools.shell.run(shell_command).await {
			Ok(result) => result,
			Err(error) => {
				let workspace_after = WSSnapshot::capture(&self.workspace.cwd)?;
				let changed_paths =
					reconcile_agent_artifacts(&mut run.ctx, &workspace_before, &workspace_after)?;

				let message = format!(
					"Shell execution failed for `{command}`: {error:#}\n\
					Observed workspace changes:\n{}",
					fmt_changed_files(&changed_paths),
				);

				eprintln!("{message}");

				run.record_error(AgentError {
					step: Some(step),
					kind: "command_execution".into(),
					message: message.clone(),
					raw_response: None,
					recoverable: true,
					timestamp: chrono::Utc::now(),
				});

				run.ctx.history.push(AgentObservation::Current {
					message: format!(
						"{message}\n\
						The shell tool did not return a completed result. \
						Inspect the error and observed changes before retrying."
					),
				});

				return Ok(());
			}
		};

		section!("SHELL RESULT");

		println!("command: {command}");
		println!("exit: {:?}", result.exit_code);

		println!(
			"stdout ({} chars, {} lines):\n{}",
			result.stdout.len(),
			result.stdout.lines().count(),
			preview_lines(&result.stdout, SHELL_OUTPUT_PREVIEW_LINES),
		);

		println!(
			"stderr ({} chars, {} lines):\n{}",
			result.stderr.len(),
			result.stderr.lines().count(),
			preview_lines(&result.stderr, SHELL_OUTPUT_PREVIEW_LINES),
		);

		let workspace_after = WSSnapshot::capture(&self.workspace.cwd)?;
		let changed_paths =
			reconcile_agent_artifacts(&mut run.ctx, &workspace_before, &workspace_after)?;

		println!(
			"FILES CHANGED BY ACTION:\n{}",
			fmt_changed_files(&changed_paths),
		);

		let success = result.exit_code == Some(0);

		run.record_command(CommandRecord {
			step,
			command: command.clone(),
			argv: None,
			exit_code: result.exit_code,
			stdout: result.stdout.clone(),
			stderr: result.stderr.clone(),
			success,
			error: if success {
				None
			} else {
				Some(format!("Command exited with status {:?}", result.exit_code,))
			},
			timestamp: chrono::Utc::now(),
		});

		if !success {
			run.record_error(AgentError {
				step: Some(step),
				kind: "command_exit".into(),
				message: format!(
					"Command `{command}` exited with status {:?}.\n\
					Stderr:\n{}",
					result.exit_code,
					preview(&result.stderr, 1000),
				),
				raw_response: None,
				recoverable: true,
				timestamp: chrono::Utc::now(),
			});
		}

		run.ctx.history.push(AgentObservation::Current {
			message: format!(
				"Runtime filesystem reconciliation after command `{command}`:\n\
				{}\n\
				These changes were observed by the runtime. Inspect and verify \
				the relevant files before claiming completion.",
				fmt_changed_files(&changed_paths),
			),
		});

		run
			.ctx
			.history
			.push(AgentObservation::RunCommand { result });

		Ok(())
	}
	async fn execute_program_action(
		&self,
		run: &mut AgentRun,
		step: usize,
		program: String,
		args: Vec<String>,
		cwd: Option<String>,
	) -> Result<()> {
		let display_command = std::iter::once(program.as_str())
			.chain(args.iter().map(String::as_str))
			.collect::<Vec<_>>()
			.join(" ");
		let workspace_before = WSSnapshot::capture(&self.workspace.cwd)?;
		let mut shell_command = ShellCommand::program(program.clone(), args.clone());
		let _requested_cwd = cwd;
		shell_command.cwd = Some(self.workspace.cwd.clone());

		let result = match self.tools.shell.run(shell_command).await {
			Ok(result) => result,
			Err(error) => {
				let workspace_after = WSSnapshot::capture(&self.workspace.cwd)?;
				let changed_paths =
					reconcile_agent_artifacts(&mut run.ctx, &workspace_before, &workspace_after)?;

				let message = format!(
					"Program execution failed for `{display_command}`: {error:#}\n\
					Observed workspace changes:\n{}",
					fmt_changed_files(&changed_paths),
				);

				run.record_error(AgentError {
					step: Some(step),
					kind: "command_execution".into(),
					message: message.clone(),
					raw_response: None,
					recoverable: true,
					timestamp: chrono::Utc::now(),
				});

				run.ctx.history.push(AgentObservation::Current {
					message: format!(
						"{message}\n\
						The program did not return a completed result. Inspect \
						the error and observed changes before choosing the next action."
					),
				});

				return Ok(());
			}
		};

		section!("PROGRAM RESULT");
		//
		// 		println!("program: {program}");
		// 		println!("args: {args:?}");
		// 		println!("exit: {:?}", result.exit_code);
		//
		// 		println!(
		// 			"stdout ({} chars, {} lines):\n{}",
		// 			result.stdout.len(),
		// 			result.stdout.lines().count(),
		// 			preview_lines(&result.stdout, SHELL_OUTPUT_PREVIEW_LINES),
		// 		);
		//
		// 		println!(
		// 			"stderr ({} chars, {} lines):\n{}",
		// 			result.stderr.len(),
		// 			result.stderr.lines().count(),
		// 			preview_lines(&result.stderr, SHELL_OUTPUT_PREVIEW_LINES),
		// 		);

		let workspace_after = WSSnapshot::capture(&self.workspace.cwd)?;
		let changed_paths =
			reconcile_agent_artifacts(&mut run.ctx, &workspace_before, &workspace_after)?;
		//
		// 		println!(
		// 			"FILES CHANGED BY ACTION:\n{}",
		// 			fmt_changed_files(&changed_paths),
		// 		);
		let success = result.exit_code == Some(0);
		run.record_command(CommandRecord {
			step,
			command: display_command.clone(),
			argv: Some(
				std::iter::once(program.clone())
					.chain(args.iter().cloned())
					.collect(),
			),
			exit_code: result.exit_code,
			stdout: result.stdout.clone(),
			stderr: result.stderr.clone(),
			success,
			error: if success {
				None
			} else {
				Some(format!("Program exited with status {:?}", result.exit_code,))
			},
			timestamp: chrono::Utc::now(),
		});
		if !success {
			run.record_error(AgentError {
				step: Some(step),
				kind: "command_exit".into(),
				message: format!(
					"Program `{display_command}` exited with status {:?}.\n\
					Stderr:\n{}",
					result.exit_code,
					preview(&result.stderr, 1000),
				),
				raw_response: None,
				recoverable: true,
				timestamp: chrono::Utc::now(),
			});
		}
		run.ctx.history.push(AgentObservation::Current {
			message: format!(
				"Runtime filesystem reconciliation after program `{display_command}`:\n\
				{}\n\
				These changes were observed by the runtime. Inspect and verify \
				the relevant files before claiming completion.",
				fmt_changed_files(&changed_paths),
			),
		});
		run
			.ctx
			.history
			.push(AgentObservation::RunCommand { result });
		Ok(())
	}
	pub fn new() -> Self {
		Self {
			id: uuid::Uuid::new_v4().to_string(),
			tools: AgentTools::default(),
			workspace: Arc::new(CtxWorkspace::default()),
			files_seen: 0,
			tokens_seen: 0,
		}
	}
	pub fn with_ctx(ctx: AgentCtx, _session: &PLSession) -> Result<Self> {
		Ok(Self {
			files_seen: 0,
			tokens_seen: 0,
			id: uuid::Uuid::new_v4().to_string(),
			tools: AgentTools::default(),
			workspace: Arc::new(ctx.workspace),
		})
	}
	pub fn with_workspace(workspace: CtxWorkspace) -> Self {
		Self {
			files_seen: 0,
			tokens_seen: 0,
			id: uuid::Uuid::new_v4().to_string(),
			tools: AgentTools::default(),
			workspace: Arc::new(workspace),
		}
	}
	pub async fn run_agent_loop(
		&self,
		task: AgentTask,
		event_tx: UnboundedSender<RuntimeEvent>,
	) -> Result<TaskResult> {
		let max_steps = 20;
		let mut ctx = AgentCtx::with_workspace(task.prompt.clone(), (*self.workspace).clone());
		ctx.task = Some(task.clone());
		ctx.todo = task
			.expected_outputs
			.iter()
			.map(|path| format!("Implement and verify {}", path.display()))
			.collect();
		let mut run = AgentRun::new(ctx);
		print_run_metrics(&mut run.clone(), self.files_seen, max_steps);
		self.emit_thinking(&task, &event_tx);
		log_run(&mut run, "Selecting agent mode");
		let mode = match self.pick_mode(&run.ctx).await {
			Ok(mode) => {
				log_run(&mut run, format!("Mode selected: {mode:?}"));
				mode
			}
			Err(error) => {
				log_run(&mut run, format!("MODE SELECTION FAILED: {error:#}"));
				return Err(error);
			}
		};
		let mode = if matches!(&mode, AgentMode::Chat) && task.expected_outputs.is_empty() {
			mode
		} else {
			AgentMode::Tool
		};

		if matches!(mode, AgentMode::Chat) {
			log_run(&mut run, "Entering chat path");

			let response = match prompt_chat(&run.ctx).await {
				Ok(response) => response,
				Err(error) => {
					log_run(&mut run, format!("CHAT FAILED: {error:#}"));
					return Err(error);
				}
			};

			log_run(
				&mut run,
				format!("Chat completed: response_chars={}", response.len()),
			);

			return Ok(TaskResult::completed_chat(task.id, run.ctx, response));
		}

		log_run(
			&mut run,
			format!(
				"Capturing baseline workspace snapshot: {}",
				self.workspace.cwd.display()
			),
		);

		let baseline = match WSSnapshot::capture(&self.workspace.cwd) {
			Ok(snapshot) => {
				log_run(&mut run, "Baseline workspace snapshot captured");
				snapshot
			}
			Err(error) => {
				log_run(&mut run, format!("BASELINE SNAPSHOT FAILED: {error:#}"));
				return Err(error.into());
			}
		};

		let mut guard = AgentGuard::default();
		log_run(
			&mut run,
			format!("Starting agent loop: max_steps={max_steps}"),
		);

		for step in 1..=max_steps {
			print_steps(&mut run, step, max_steps, self.tokens_seen, self.files_seen);
			let action = match self.select_action(&mut run, step).await {
				Ok(Some(action)) => {
					log_run(
						&mut run,
						format!("STEP {step}: selected action: {action:?}"),
					);
					action
				}
				Ok(None) => {
					log_run(
						&mut run,
						format!("STEP {step}: action selection exhausted retries"),
					);

					return Ok(TaskResult::failed(
						task.id,
						run.ctx,
						"Action selection failed",
						Some("Too many consecutive invalid actions.".into()),
						run.commands,
						run.errors,
					));
				}
				Err(error) => {
					log_run(
						&mut run,
						format!("STEP {step}: ACTION SELECTION ERROR: {error:#}"),
					);
					return Err(error);
				}
			};
			match self.guard_action(&action, &run, &mut guard) {
				Ok(GuardDecision::Allow) => {
					log_run(&mut run, format!("STEP {step}: guard allowed action"));
				}
				Ok(GuardDecision::Reject(reason)) => {
					log_run(
						&mut run,
						format!("STEP {step}: GUARD REJECTED ACTION: {reason}"),
					);

					self.reject_action(&mut run, step, reason);
					continue;
				}
				Err(error) => {
					log_run(&mut run, format!("STEP {step}: GUARD ERROR: {error:#}"));
					return Err(error);
				}
			}
			match action {
				AgentAction::Current { message } => {
					log_run(
						&mut run,
						format!("STEP {step}: CURRENT message_chars={}", message.len()),
					);
					self.handle_current(&task, &event_tx, &mut run, message);
					log_run(&mut run, format!("STEP {step}: CURRENT handled"));
				}
				AgentAction::Context { path } => {
					log_run(&mut run, format!("STEP {step}: CONTEXT path={path:?}"));
					self.handle_context(&task, &event_tx, &mut run, path);
					log_run(&mut run, format!("STEP {step}: CONTEXT handled"));
				}
				AgentAction::RunCommand { command } => {
					log_run(
						&mut run,
						format!("STEP {step}: SHELL START command={command:?}"),
					);
					log_step(&mut run, step, format!("SHELL START command={command:?}"));
					match self.execute_shell_action(&mut run, step, command).await {
						Ok(()) => log_step(&mut run, step, "SHELL HANDLER COMPLETE"),
						Err(error) => {
							log_step(&mut run, step, format!("SHELL HANDLER FAILED: {error:#}"));
							return Err(error);
						}
					}
				}
				AgentAction::RunProgram { program, args, cwd } => {
					log_run(
						&mut run,
						format!("STEP {step}: PROGRAM START program={program:?} args={args:?} cwd={cwd:?}"),
					);
					match self
						.execute_program_action(&mut run, step, program, args, cwd)
						.await
					{
						Ok(()) => log_step(&mut run, step, "PROGRAM HANDLER COMPLETE"),
						Err(error) => {
							log_step(&mut run, step, format!("PROGRAM HANDLER FAILED: {error:#}"));
							return Err(error);
						}
					}
				}
				AgentAction::Finish { message } => {
					log_run(
						&mut run,
						format!("STEP {step}: FINISH requested message={message:?}"),
					);
					match self.try_finish(task.id, &mut run, &baseline, step, message) {
						Ok(Some(result)) => {
							log_run(&mut run, format!("STEP {step}: FINISH VALIDATED"));
							self.emit_finished(&event_tx, result.clone());
							return Ok(result);
						}
						Ok(None) => {
							log_run(
								&mut run,
								format!("STEP {step}: FINISH REJECTED; continuing loop"),
							);
						}
						Err(error) => {
							log_run(
								&mut run,
								format!("STEP {step}: FINISH VALIDATION ERROR: {error:#}"),
							);
							return Err(error);
						}
					}
				}
			}
		}
		log_run(
			&mut run,
			format!("STEP LIMIT REACHED: max_steps={max_steps}"),
		);
		Ok(self.fail_step_limit(task.id, run, max_steps))
	}
}
impl Default for Agent {
	fn default() -> Self {
		Self::new()
	}
}
impl AgentRun {
	pub fn new(ctx: AgentCtx) -> Self {
		Self {
			ctx,
			commands: Vec::new(),
			errors: Vec::new(),
		}
	}
	pub fn record_command(&mut self, record: CommandRecord) {
		self.commands.push(record);
	}
	pub fn record_error(&mut self, error: AgentError) {
		self.errors.push(error);
	}
	pub fn record_observation(&mut self, observation: AgentObservation) {
		self.ctx.history.push(observation);
	}
	pub fn record_log(&mut self, message: impl Into<String>) {
		self.ctx.logs.push(message.into());
	}
	pub fn step_count(&self) -> usize {
		self.commands.len() + self.errors.len()
	}
	pub fn has_successful_command(&self) -> bool {
		self.commands.iter().any(|command| command.success)
	}
	pub fn has_errors(&self) -> bool {
		!self.errors.is_empty()
	}
	pub fn last_command(&self) -> Option<&CommandRecord> {
		self.commands.last()
	}
	pub fn last_error(&self) -> Option<&AgentError> {
		self.errors.last()
	}
	pub fn into_context(self) -> AgentCtx {
		self.ctx
	}
}
impl AgentTask {
	pub fn new(prompt: String) -> Self {
		Self {
			id: Uuid::new_v4(),
			prompt,
			expected_outputs: Vec::new(),
			recovery_prompt: None,
		}
	}
	pub fn with_expected_outputs(mut self, paths: Vec<PathBuf>) -> Self {
		self.expected_outputs = paths;
		self
	}
	pub fn with_recovery_prompt(mut self, prompt: impl Into<String>) -> Self {
		self.recovery_prompt = Some(prompt.into());
		self
	}
	pub fn from_session(session: &PLSession) -> Result<Self> {
		Ok(Self {
			id: Uuid::new_v4(),
			prompt: session.prompt.clone(),
			expected_outputs: Vec::new(),
			recovery_prompt: None,
		})
	}
}

impl AgentCtx {
	pub fn new(user_prompt: String) -> Self {
		Self {
			prompt: Some(user_prompt.clone()),
			task: Some(AgentTask::new(user_prompt)),
			workspace: CtxWorkspace::default(),
			history: Vec::new(),
			artifacts: Vec::new(),
			logs: Vec::new(),
			spawned_tasks: Vec::new(),
			todo: Vec::new(),
		}
	}
	pub fn init() -> Self {
		Self {
			prompt: None,
			task: None,
			workspace: CtxWorkspace::default(),
			history: Vec::new(),
			artifacts: Vec::new(),
			logs: Vec::new(),
			spawned_tasks: Vec::new(),
			todo: Vec::new(),
		}
	}
	pub fn with_workspace(user_prompt: String, workspace: CtxWorkspace) -> Self {
		Self {
			prompt: Some(user_prompt.clone()),
			task: Some(AgentTask::new(user_prompt)),
			workspace,
			history: Vec::new(),
			artifacts: Vec::new(),
			logs: Vec::new(),
			spawned_tasks: Vec::new(),
			todo: Vec::new(),
		}
	}
	pub fn from_session(session: &PLSession) -> Result<Self> {
		let workspace = CtxWorkspace::from_session(session)?;
		Self::from_session_with_workspace(session, &workspace)
	}
	pub fn from_session_with_workspace(
		session: &PLSession,
		workspace: &CtxWorkspace,
	) -> Result<Self> {
		Ok(Self {
			prompt: Some(session.prompt.clone()),
			task: Some(AgentTask::new(session.prompt.clone())),
			workspace: workspace.clone(),
			history: Vec::new(),
			artifacts: Vec::new(),
			logs: Vec::new(),
			spawned_tasks: Vec::new(),
			todo: Vec::new(),
		})
	}
}

impl AgentCompletionValidation {
	pub fn is_valid(&self) -> bool {
		self.errors.is_empty()
	}

	pub fn errors(&self) -> &[String] {
		&self.errors
	}
}

impl TryFrom<LlmAction> for AgentAction {
	type Error = anyhow::Error;

	fn try_from(raw: LlmAction) -> Result<Self> {
		match raw.action.trim() {
			"run_command" => match (raw.command, raw.args) {
				(Some(command), None) if !command.trim().is_empty() => Ok(Self::RunCommand { command }),

				(None, Some(args)) if !args.is_empty() => {
					let mut args = args.into_iter();
					let program = args
						.next()
						.filter(|program| !program.trim().is_empty())
						.ok_or_else(|| {
							anyhow::anyhow!("run_command args must start with a non-empty program")
						})?;

					Ok(Self::RunProgram {
						program,
						args: args.collect(),
						cwd: raw.cwd,
					})
				}

				(Some(_), Some(_)) => {
					anyhow::bail!("ambiguous run_command: provide command or args, not both")
				}

				(Some(_), None) => {
					anyhow::bail!("run_command requires a non-empty command")
				}

				(None, Some(_)) => {
					anyhow::bail!("run_command args must not be empty")
				}

				(None, None) => {
					anyhow::bail!("run_command requires either command or args")
				}
			},

			"finish" => {
				let message = raw
					.message
					.filter(|message| !message.trim().is_empty())
					.ok_or_else(|| anyhow::anyhow!("finish requires a non-empty message"))?;

				Ok(Self::Finish { message })
			}

			action => {
				anyhow::bail!("unsupported action: {action:?}")
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct Agent {
	pub id: String,
	pub tools: AgentTools,
	pub workspace: Arc<CtxWorkspace>,
	pub tokens_seen: u128,
	pub files_seen: u128,
	// pub tokens_seen: u128,
}

#[derive(Clone, Debug)]
pub struct AgentBus {
	pub tx: UnboundedSender<AgentEvent>,
	pub event_tx: UnboundedSender<RuntimeEvent>,
}

pub struct AgentCompletionValidation {
	pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AgentCtx {
	pub prompt: Option<String>,
	pub task: Option<AgentTask>,
	pub workspace: CtxWorkspace,
	pub history: Vec<AgentObservation>,
	pub artifacts: Vec<Artifact>,
	pub logs: Vec<String>,
	pub spawned_tasks: Vec<AgentTask>,
	#[serde(default)]
	pub todo: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentError {
	/// One-based agent step where the error occurred, if known.
	pub step: Option<usize>,

	/// Category of failure, e.g. "action_parse", "command", or "tool".
	pub kind: String,

	/// Human-readable error description.
	pub message: String,

	/// Raw model response, if the error occurred while parsing an action.
	pub raw_response: Option<String>,

	/// Whether the error was recoverable.
	pub recoverable: bool,

	/// Timestamp when the error was recorded.
	pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default)]
pub struct AgentGuard {
	consecutive_no_progress: usize,
	recent_commands: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
	pub ctx: AgentCtx,
	pub commands: Vec<CommandRecord>,
	pub errors: Vec<AgentError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AgentTask {
	pub id: Uuid,
	pub prompt: String,
	#[serde(default)]
	pub expected_outputs: Vec<PathBuf>,
	#[serde(default)]
	pub recovery_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
	/// One-based execution order within this agent run.
	pub step: usize,

	/// Program and arguments for direct process execution, if applicable.
	#[serde(default)]
	pub argv: Option<Vec<String>>,

	/// The command submitted to the shell.
	pub command: String,

	/// Process exit code, if one was produced.
	pub exit_code: Option<i32>,

	/// Captured standard output.
	pub stdout: String,

	/// Captured standard error.
	pub stderr: String,

	/// Whether the command completed successfully.
	pub success: bool,

	/// Error encountered while launching or executing the command, if any.
	pub error: Option<String>,

	/// Timestamp when the command record was created.
	pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
pub struct ExecutionRecord {
	pub step: usize,
	pub action: String,
	pub cwd: Option<PathBuf>,
	pub outcome: ExecutionOutcome,
	pub exit_code: Option<i32>,
	pub stdout: String,
	pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAction {
	pub action: String,
	#[serde(default)]
	pub command: Option<String>,
	#[serde(default)]
	pub message: Option<String>,
	#[serde(default)]
	pub args: Option<Vec<String>>,
	#[serde(default)]
	pub cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LlmMode {
	pub mode: String,
}
