use super::{SdlcEvent as E, *};

use crate::{agent::fmt_manifest, prelude::*};

use profuse::prelude::{r#fn::sleep, r#trait::Ctx, *};

// Steps to complete the pipeline
trait TraitPL {
	type Stage: StageTrait;
	fn id(&self) -> &PipelineId;
	fn name(&self) -> &'static str;
	fn description(&self) -> &'static str {
		""
	}
	fn stages(&self) -> &[Self::Stage];
	fn stage(&self, name: &str) -> Option<&Self::Stage> {
		self.stages().iter().find(|stage| stage.name() == name)
	}
	fn first_stage(&self) -> Option<&Self::Stage> {
		self.stages().first()
	}
	fn next_stage(&self, current: &str) -> Option<&Self::Stage> {
		let stages = self.stages();
		let index = stages.iter().position(|stage| stage.name() == current)?;
		stages.get(index + 1)
	}
	fn previous_stage(&self, current: &str) -> Option<&Self::Stage> {
		let stages = self.stages();
		let index = stages.iter().position(|stage| stage.name() == current)?;
		index.checked_sub(1).and_then(|index| stages.get(index))
	}
	fn validate(&self) -> Result<()> {
		let stages = self.stages();
		anyhow::ensure!(!stages.is_empty(), "Pipeline has no stages");
		for (index, stage) in stages.iter().enumerate() {
			anyhow::ensure!(
				!stages[..index]
					.iter()
					.any(|previous| previous.name() == stage.name()),
				"Duplicate stage name: {}",
				stage.name(),
			);
		}
		Ok(())
	}
}

pub fn read_from_session(name: &str, session: &PLSession) -> anyhow::Result<String> {
	read_from_disk(session.dir.join(name))
}
pub fn read_from_disk(path: impl AsRef<Path>) -> anyhow::Result<String> {
	std::fs::read_to_string(path).map_err(Into::into)
}

impl Pipeline {
	fn transition(&mut self, next: Stage) -> Result<()> {
		let valid = matches!(
			(self.stage, next),
			(Stage::Intent, Stage::Spec)
				| (Stage::Spec, Stage::Plan)
				| (Stage::Plan, Stage::Build)
				| (Stage::Build, Stage::QA)
				| (Stage::QA, Stage::Build)
				| (Stage::QA, Stage::Complete)
		);
		if !valid {
			anyhow::bail!("invalid SDLC transition: {:?} -> {:?}", self.stage, next);
		}
		self.stage = next;
		Ok(())
	}
	fn is_passing(&self, checks: &[CheckResult], evaluations: &[Metric]) -> bool {
		checks.iter().all(|check| check.passed)
			&& evaluations.iter().all(|evaluation| evaluation.passed)
	}
	fn retry(&mut self, _stage: Stage) -> Result<()> {
		Ok(())
	}
	pub fn attempt_bump(&mut self) -> u32 {
		self.stage_attempt += 1;
		self.stage_attempt
	}
	pub fn attempt_reset(&mut self) {
		self.stage_attempt = 1;
	}
	pub fn summarize_goal(_intent: &str) -> Result<String> {
		Ok(String::from("create-sdlc-pipeline"))
	}
	pub async fn wait_for_intervention(
		&mut self,
		stage: Stage,
		attempt: Attempt,
		reason: String,
		input_rx: &mut tokio::sync::mpsc::UnboundedReceiver<SdlcInput>,
	) -> Result<Intervention> {
		log::emit(E::InterventionRequired {
			stage,
			attempt,
			reason,
		});
		log::emit(E::PhaseChanged {
			stage,
			phase: Phase::AwaitingHuman,
		});
		let input = input_rx
			.recv()
			.await
			.ok_or_else(|| anyhow!("SDLC input channel closed"))?;
		match input {
			SdlcInput::Abort => {
				log::emit(E::InterventionResolved {
					stage,
					action: "abort".into(),
				});
				Ok(Intervention::Abort)
			}
			SdlcInput::Human(string) => {
				log::emit(E::HumanInput {
					stage,
					input: string.clone(),
				});
				log::emit(E::InterventionResolved {
					stage,
					action: "human input provided".into(),
				});
				Ok(Intervention::Human(string))
			}
			SdlcInput::ProvideContext(context) => {
				log::emit(E::InterventionResolved {
					stage,
					action: "context provided".into(),
				});
				Ok(Intervention::ProvideContext(context))
			}
			SdlcInput::Retry => {
				log::emit(E::InterventionResolved {
					stage,
					action: "retry".into(),
				});
				Ok(Intervention::Retry)
			}
			SdlcInput::Reviewed => {
				log::emit(E::InterventionResolved {
					stage,
					action: "reviewed".into(),
				});
				Ok(Intervention::Reviewed)
			}
			SdlcInput::Revision { evaluation } => {
				log::emit(E::InterventionResolved {
					stage,
					action: "revision".into(),
				});
				Ok(Intervention::Revision { evaluation })
			}
		}
	}
}

impl<'a, C: Ctx> _Runner<'a, C> {
	fn transition(&mut self, next: Stage) -> Result<()> {
		self.pipeline.transition(next)?;
		self.runtime.session.stage = next;
		self.runtime.session.time_updated = Utc::now();
		self.runtime.persist_session()?;
		Ok(())
	}
	fn write(&mut self, content: &str) -> Result<()> {
		let log_dir = self.stage_dir();
		let stage = self.pipeline.stage.clone();
		let filename = format!("{stage:?}").to_lowercase() + ".md";
		let output_path = log_dir.join(&filename);
		tracing::info!(
				stage = ?stage,
				log_dir = %log_dir.display(),
				filename = %filename,
				output_path = %output_path.display(),
				content_len = content.len(),
				"SDLC artifact write"
		);
		std::fs::write(output_path, content).context("writing generated SDLC artifact")?;
		Ok(())
	}
	fn next_attempt(&mut self, stage: Stage) -> Result<Attempt> {
		let session = self.runtime.session()?;
		let max = DEFAULT_LOOP_TRIES;

		let number = session
			.stages
			.iter()
			.filter(|record| record.stage == stage)
			.map(|record| record.attempt.number)
			.max()
			.map_or(1, |number| number + 1);

		Ok(Attempt { stage, number, max })
	}
	fn evaluate_checks(&self, checks: &[CheckResult]) -> Vec<Metric> {
		checks
			.iter()
			.map(|check| Metric {
				name: check.name.clone(),
				passed: check.passed,
				score: if check.passed { 1.0 } else { 0.0 },
				confidence: 1.0,
				explanation: match &check.output {
					Some(output) if !output.is_empty() => output.clone(),
					_ => {
						if check.passed {
							"Check passed.".into()
						} else {
							"Check failed.".into()
						}
					}
				},
			})
			.collect()
	}
	fn session(&self) -> &PLSession {
		&self.runtime.session
	}
	fn stage_dir(&self) -> PathBuf {
		self.session().dir.clone()
	}

	async fn apply(
		&mut self,
		outcome: AttemptOutcome,
		decision: Decision,
		input_rx: &mut UnboundedReceiver<SdlcInput>,
		pending_input: &mut Option<SdlcInput>,
	) -> Result<RunControl> {
		let stage = outcome.stage();
		let attempt = outcome.attempt();
		match decision {
			Decision::AwaitHuman => {
				log::emit(E::PhaseChanged {
					stage,
					phase: Phase::AwaitingHuman,
				});
				let intervention = self
					.wait_for_intervention(
						stage,
						attempt,
						format!("Stage {stage:?} requires human intervention"),
						input_rx,
					)
					.await?;
				match intervention {
					Intervention::Retry => {
						if attempt.number >= attempt.max {
							return Ok(RunControl::Exit(PipelineStatus::Failed));
						}
						self.pipeline.attempt_bump();
						self.handle_retry(stage, attempt).await?;
						Ok(RunControl::RetryStage)
					}
					Intervention::Revision { evaluation } => {
						*pending_input = Some(SdlcInput::Revision { evaluation });
						self.pipeline.retry(stage)?;
						Ok(RunControl::RetryStage)
					}
					Intervention::Revise => {
						self
							.handle_revision(stage, attempt, outcome, input_rx, pending_input)
							.await?;
						Ok(RunControl::RetryStage)
					}
					Intervention::Reviewed => Ok(RunControl::RetryStage),
					Intervention::ProvideContext(context) => {
						*pending_input = None;
						self
							.runtime
							.persist_progress(&format!("Human provided context: {context}"))?;
						Ok(RunControl::RetryStage)
					}
					Intervention::Abort => {
						log::emit(E::Failed {
							stage: Some(stage),
							error: "aborted by user".into(),
						});
						Ok(RunControl::Exit(PipelineStatus::Interrupted))
					}
					Intervention::Human(_) => Ok(RunControl::Exit(PipelineStatus::Interrupted)),
				}
			}
			Decision::Complete => {
				log::emit(E::PhaseChanged {
					stage,
					phase: Phase::Completed,
				});
				self
					.runtime
					.persist_progress(&format!("{} completed", stage))?;
				log::emit(E::Completed);
				Ok(RunControl::Exit(PipelineStatus::Complete))
			}
			Decision::Continue => {
				let next = stage
					.next()
					.ok_or_else(|| anyhow!("Stage {stage:?} has no next stage"))?;
				self.transition(next)?;
				log::emit(E::StageTransitioned {
					from: stage,
					to: next,
				});
				Ok(RunControl::Continue)
			}
			Decision::Exit => {
				log::emit(E::Failed {
					stage: Some(stage),
					error: format!("Stage {stage:?} exited on attempt {}", attempt.number),
				});
				Ok(RunControl::Exit(PipelineStatus::Interrupted))
			}
			Decision::Fail => {
				log::emit(E::Failed {
					stage: Some(stage),
					error: format!("Stage {stage:?} failed on attempt {}", attempt.number),
				});

				self
					.runtime
					.persist_progress(&format!("{} failed", stage))?;
				Ok(RunControl::Exit(PipelineStatus::Failed))
			}
			Decision::Retry => {
				if attempt.number >= attempt.max {
					tracing::warn!(
						stage = ?stage,
						attempt = attempt.number,
						max = attempt.max,
						"maximum stage attempts reached"
					);

					log::emit(E::Failed {
						stage: Some(stage),
						error: format!(
							"Stage {stage:?} exhausted maximum attempts ({})",
							attempt.max
						),
					});

					self.runtime.persist_progress(&format!(
						"{} exhausted maximum attempts ({})",
						stage, attempt.max
					))?;

					return Ok(RunControl::Exit(PipelineStatus::Failed));
				}

				let next_attempt = attempt.number + 1;

				self.runtime.persist_progress(&format!(
					"Retrying stage {} (attempt {}/{})",
					stage, next_attempt, attempt.max,
				))?;
				self.pipeline.attempt_bump();
				self.handle_retry(stage, attempt).await?;
				Ok(RunControl::RetryStage)
			}
			Decision::Revise => {
				self
					.runtime
					.persist_progress(&format!("Revising stage {}", stage))?;

				Ok(RunControl::RetryStage)
			}
		}
	}
	async fn _await_human(
		&mut self,
		stage: Stage,
		attempt: Attempt,
		input_rx: &mut UnboundedReceiver<SdlcInput>,
		pending_input: &mut Option<SdlcInput>,
	) -> Result<RunControl> {
		log::emit(E::PhaseChanged {
			stage,
			phase: Phase::AwaitingHuman,
		});
		log::emit(E::Activity {
			stage,
			attempt,
			message: "Waiting for human input".into(),
		});
		let input = input_rx.recv().await.context("SDLC input channel closed")?;
		match input {
			SdlcInput::Abort => {
				log::emit(E::Failed {
					stage: Some(stage),
					error: "aborted by user".into(),
				});

				Ok(RunControl::Exit(PipelineStatus::Interrupted))
			}
			SdlcInput::Human(input) => {
				*pending_input = Some(SdlcInput::Human(input));
				self.pipeline.retry(stage)?;

				Ok(RunControl::Continue)
			}
			SdlcInput::ProvideContext(context) => {
				*pending_input = Some(SdlcInput::ProvideContext(context));
				self.pipeline.retry(stage)?;

				Ok(RunControl::Continue)
			}
			SdlcInput::Revision { evaluation } => {
				*pending_input = Some(SdlcInput::Revision { evaluation });
				self.pipeline.retry(stage)?;
				Ok(RunControl::Continue)
			}
			SdlcInput::Retry => {
				self.pipeline.retry(stage)?;
				Ok(RunControl::Continue)
			}
			SdlcInput::Reviewed => {
				let next = stage
					.next()
					.ok_or_else(|| anyhow!("Stage {stage:?} has no next stage"))?;
				self.transition(next)?;
				log::emit(E::StageTransitioned {
					from: stage,
					to: next,
				});
				Ok(RunControl::Continue)
			}
		}
	}
	async fn evaluate(&self, execution: &Execution) -> Result<QACheck> {
		tracing::info!("evaluate pipeline runner {:?}", execution);
		let checks: Vec<CheckResult> = self.runtime.run_checks(execution.stage).await?;
		let semantic = self.runtime.qa.evaluate(execution).await?;
		Ok(semantic.with_evaluations(checks))
	}
	async fn execute(
		&mut self,
		stage: Stage,
		attempt: Attempt,
		pending_input: &mut Option<SdlcInput>,
	) -> Result<AttemptOutcome> {
		let _result = log(&E::RunAttempt {
			stage,
			phase: Phase::Executing,
		});
		let input = match pending_input.take() {
			Some(SdlcInput::Revision { evaluation }) => StageInput::Revision { evaluation },
			Some(input) => {
				*pending_input = Some(input);
				StageInput::Initial
			}
			None => StageInput::Initial,
		};
		let execution = match self.route_execution(stage, attempt, input).await {
			Ok(execution) => execution,
			Err(error) => {
				return Ok(AttemptOutcome::ExecutionFailed {
					stage,
					attempt,
					error,
				});
			}
		};
		self.handle_evaluation(execution, attempt).await
	}
	async fn execute_attempt_inner(&mut self, _stage: Stage) -> Result<AttemptOutcome> {
		todo!("move existing implementation here")
	}

	async fn handle_outcome(&self, outcome: &AttemptOutcome) -> Result<Decision> {
		if std::env::var_os("SDLC_FORCE_EXIT").is_some() {
			section_info!(
				"decision",
				metrics: [
					"override" => "SDLC_FORCE_EXIT",
					"decision" => "Exit",
				],
				"" => "Forced exit",
			);
			return Ok(Decision::Exit);
		}

		if std::env::var_os("SDLC_FORCE_CONTINUE").is_some() {
			section_info!(
				"decision",
				metrics: [
					"override" => "SDLC_FORCE_CONTINUE",
					"decision" => "Continue",
				],
				"" => "Forced continue",
			);
			return Ok(Decision::Continue);
		}

		let (decision, stage, attempt, max_attempts, outcome_name, details) = match outcome {
			AttemptOutcome::EvaluationFailed { execution, .. } => {
				let attempt = &execution.attempt;
				let decision = if attempt.number < attempt.max {
					Decision::Retry
				} else {
					Decision::AwaitHuman
				};

				(
					decision,
					attempt.stage,
					attempt.number,
					attempt.max,
					"EvaluationFailed",
					"Evaluation failed".to_string(),
				)
			}
			AttemptOutcome::Progress { execution, .. } => {
				let decision = match &execution.result {
					RunResult::Verification(verification) if verification.passed => Decision::Continue,
					RunResult::Verification(_) => Decision::Retry,
					_ if execution.stage == Stage::Complete => Decision::Complete,
					_ => Decision::Continue,
				};

				let details = match &execution.result {
					RunResult::Verification(verification) => {
						format!("Verification passed={}", verification.passed)
					}
					_ => "Execution completed".to_string(),
				};

				(
					decision,
					execution.stage,
					execution.attempt.number,
					execution.attempt.max,
					"Complete",
					details,
				)
			}
			AttemptOutcome::NeedsRevision {
				execution,
				evaluation,
			} => {
				let attempt = &execution.attempt;
				let decision = if attempt.number < attempt.max {
					Decision::Revise
				} else {
					Decision::AwaitHuman
				};

				(
					decision,
					execution.stage,
					attempt.number,
					attempt.max,
					"NeedsRevision",
					format!(
						"Revision required\nscore => {:.2}\nconfidence => {:.2}",
						evaluation.score, evaluation.confidence,
					),
				)
			}
			AttemptOutcome::ExecutionFailed { attempt, error, .. } => {
				let decision = if attempt.number < attempt.max {
					Decision::Retry
				} else {
					Decision::AwaitHuman
				};

				(
					decision,
					attempt.stage,
					attempt.number,
					attempt.max,
					"ExecutionFailed",
					format!("Execution error => {error}\nError debug => {error:?}"),
				)
			}
		};

		section_info!(
			"SDLC decision",
			metrics: [
				"outcome" => outcome_name,
				"stage" => format!("{stage:?}"),
				"attempt" => format!("{attempt}/{max_attempts}"),
				"decision" => format!("{decision:?}"),
			],
			"details" => details,
		);

		Ok(decision)
	}
	async fn handle_retry(&mut self, stage: Stage, attempt: Attempt) -> Result<()> {
		let next_attempt = Attempt {
			stage,
			number: attempt.number + 1,
			max: attempt.max,
		};
		log::emit(E::StageRunRetry {
			stage,
			number: next_attempt.number,
		});
		log::emit(E::PhaseChanged {
			stage,
			phase: Phase::Retrying,
		});
		self.pipeline.retry(stage)?;
		Ok(())
	}
	async fn handle_evaluation(
		&mut self,
		execution: Execution,
		attempt: Attempt,
	) -> Result<AttemptOutcome> {
		let stage = execution.stage;
		log::emit(E::PhaseChanged {
			stage,
			phase: Phase::Evaluating,
		});
		log::emit(E::EvaluationStarted { stage });
		match self.evaluate(&execution).await {
			Ok(evaluation) => {
				log::emit(E::Evaluated {
					stage,
					score: evaluation.score,
					confidence: evaluation.confidence,
					passed: evaluation.passed,
				});
				let _result = self.runtime.persist_evaluation(&evaluation, attempt);
				if evaluation.passed {
					Ok(AttemptOutcome::Progress {
						execution,
						evaluation,
					})
				} else {
					Ok(AttemptOutcome::NeedsRevision {
						execution,
						evaluation,
					})
				}
			}
			Err(error) => Ok(AttemptOutcome::EvaluationFailed { execution, error }),
		}
	}
	async fn handle_revision(
		&mut self,
		stage: Stage,
		attempt: Attempt,
		_outcome: AttemptOutcome,
		input_rx: &mut UnboundedReceiver<SdlcInput>,
		pending_input: &mut Option<SdlcInput>,
	) -> Result<()> {
		match self
			.wait_for_intervention(
				stage,
				attempt,
				String::from("Stage needs revision"),
				input_rx,
			)
			.await?
		{
			Intervention::Abort => {
				log::emit(E::Failed {
					stage: Some(stage),
					error: "aborted by user".into(),
				});
			}
			Intervention::Human(input) => {
				*pending_input = Some(SdlcInput::Human(input));
				self.pipeline.retry(stage)?;
			}
			Intervention::ProvideContext(context) => {
				*pending_input = Some(SdlcInput::ProvideContext(context));
				self.pipeline.retry(stage)?;
			}
			Intervention::Retry => {
				self.pipeline.retry(stage)?;
			}
			Intervention::Reviewed => {
				let next = stage
					.next()
					.ok_or_else(|| anyhow!("Stage {stage:?} has no next stage"))?;
				self.transition(next)?;
				log::emit(E::StageTransitioned {
					from: stage,
					to: next,
				});
			}
			Intervention::Revise => {
				self.pipeline.retry(stage)?;
			}
			Intervention::Revision { evaluation } => {
				*pending_input = Some(SdlcInput::Revision { evaluation });
				self.pipeline.retry(stage)?;
			}
		}
		Ok(())
	}

	async fn on_intent(&mut self, input: StageInput) -> Result<RunResult> {
		let goal = self.session().prompt.clone();
		if self.pipeline.stage != Stage::Intent {
			return Err(anyhow!(
				"cannot execute Intent stage while pipeline is at {:?}",
				self.pipeline.stage
			));
		}
		let prompt = p::gen_intent(&goal)?;
		std::fs::write(ah_log_file("prompt.intent.md"), &prompt)
			.context("Intent: writing prompt debug file")?;
		let generated = self
			.runtime
			.system
			.runtime
			.generate(&prompt)
			.await
			.context("Intent: generating artifact")?;
		if generated.trim().is_empty() {
			return Err(anyhow!("generated Intent artifact is empty"));
		}
		self
			.write(&generated)
			.context("Intent: writing generated artifact")?;
		self
			.runtime
			.persist_progress("Intent stage completed")
			.context("Intent: persisting progress")?;
		Ok(RunResult::Intent)
	}
	async fn on_spec(&mut self, input: StageInput) -> Result<RunResult> {
		if self.pipeline.stage != Stage::Spec {
			return Err(anyhow!(
				"cannot execute Spec stage while pipeline is at {:?}",
				self.pipeline.stage
			));
		}
		let intent = self.runtime.session_read("intent.md")?;
		let is_revision = matches!(input, StageInput::Revision { .. });
		let prompt = match input {
			StageInput::Initial => p::gen_spec(&intent)?,
			StageInput::Revision { evaluation } => {
				let spec = self.runtime.session_read("spec.md")?;
				if spec.trim().is_empty() {
					return Err(anyhow!("Cannot revise Spec because spec.md is empty"));
				}
				p::revise_spec(&intent, &spec, &evaluation)?
			}
		};
		if prompt.trim().is_empty() {
			return Err(anyhow!("generated Spec prompt is empty"));
		}
		std::fs::write(ah_log_file("prompt.spec.md"), &prompt)
			.context("writing Spec prompt debug file")?;
		let generated = self
			.runtime
			.system
			.runtime
			.generate(&prompt)
			.await
			.context("generating Spec artifact from Intent and prompt")?;
		if generated.trim().is_empty() {
			return Err(anyhow!("generated Spec artifact is empty"));
		}
		self.write(&generated)?;
		self.runtime.persist_progress(if is_revision {
			"Spec revision completed"
		} else {
			"Spec stage completed"
		})?;
		Ok(RunResult::Spec)
	}
	async fn on_plan(&mut self, input: StageInput) -> Result<RunResult> {
		if self.pipeline.stage != Stage::Plan {
			return Err(anyhow!(
				"cannot execute Plan stage while pipeline is at {:?}",
				self.pipeline.stage
			));
		}

		let intent = self.runtime.session_read("intent.md")?;
		let spec = self.runtime.session_read("spec.md")?;
		tracing::info!(
			intent_len = intent.len(),
			spec_len = spec.len(),
			"on_plan: inputs loaded"
		);
		let prompt = p::gen_plan(&intent, &spec)?;
		tracing::info!(prompt_len = prompt.len(), "on_plan: prompt generated");
		if prompt.trim().is_empty() {
			return Err(anyhow!("Generated Plan prompt is empty"));
		}
		std::fs::write(ah_log_file("prompt.plan.md"), &prompt)
			.context("writing plan prompt debug file")?;
		tracing::info!("on_plan: runtime.generate ENTER");
		let generated = self
			.runtime
			.system
			.runtime
			.generate(&prompt)
			.await
			.context("generating Plan artifact from Intent and Spec")?;

		tracing::info!(
			generated_len = generated.len(),
			"on_plan: runtime.generate EXIT"
		);
		// Keep the rest of your existing on_plan implementation.
		tracing::info!(
			generated_len = prompt.len(),
			has_inventory = prompt.contains("Machine-Readable Output Inventory"),
			has_build_contract = prompt.contains("Machine-Readable Build Contract"),
			generated_start = %prompt.chars().take(300).collect::<String>(),
			"Plan: inspect prompt artifact"
		);
		let generated = self
			.runtime
			.system
			.runtime
			.generate(&prompt)
			.await
			.context("generating Plan artifact from Intent and Spec")?;
		if generated.trim().is_empty() {
			return Err(anyhow!("generated Plan artifact is empty"));
		}
		tracing::info!(
			generated_len = generated.len(),
				has_inventory = generated.contains("Machine-Readable Output Inventory"),
				has_build_contract = generated.contains("Machine-Readable Build Contract"),
				generated_start = %generated.chars().take(300).collect::<String>(),
				"Plan: inspect generated artifact"
		);
		std::fs::write(ah_log_file("plan.md"), &generated)
			.context("writing generated Plan debug file")?;
		let expected_outputs = p::expected_outputs_from_plan(&generated)
			.context("generated Plan has no valid expected-output inventory")?;
		if expected_outputs.is_empty() {
			return Err(anyhow!("generated Plan has no expected output files"));
		}
		self
			.write(&generated)
			.context("writing generated Plan to the SDLC session")?;
		self.runtime.persist_progress("Plan stage completed")?;
		Ok(RunResult::Plan)
	}
	async fn on_build(&mut self, _input: StageInput) -> Result<RunResult> {
		// SDLC_FORCE_CONTINUE=1 SDLC_PLAIN=1 cargo run --bin sdlc --features sdlc
		// SDLC_RESUME=1 SDLC_PLAIN=1 cargo run --bin sdlc --features sdlc
		log::emit(E::BuildBegin);
		let (session_dir, intent, spec, plan) = self.build_phase_prepare().await?;
		let workspace = self.runtime.session.workspace_owned();
		let workspace_before =
			WSSnapshot::capture(&workspace).context("capturing workspace before manifest Build")?;
		let git_status = workspace_before.git_status.trim();
		let changed_files: Vec<_> = git_status.lines().collect();
		section_info!(
			"workspace",
			metrics: [
				"files" => self.runtime.system.runtime.workspace.files.len(),
				"git" => if git_status.is_empty() { "clean" } else { "dirty" },
				"build_mode" => "manifest",
			],
			"" => changed_files.join("\n"),
		);
		let manifest = self
			.build_phase_assess(&session_dir, &intent, &spec, &plan)
			.await?;
		if manifest.files.is_empty() {
			return Err(anyhow!(
				"Build assessment returned an empty manifest; refusing to execute"
			));
		}
		println!("{}", fmt_manifest(&manifest));
		std::io::stdout().flush()?;
		let execution = self
			.build_phase_execute(
				&session_dir,
				&workspace,
				&workspace_before,
				&intent,
				&spec,
				&plan,
				&manifest,
			)
			.await?;

		// Phase 3: Verify -> inspect actual resulting files.
		let verification = self
			.build_phase_attest(
				&session_dir,
				&workspace,
				&workspace_before,
				&intent,
				&spec,
				&plan,
				&manifest,
				&execution,
			)
			.await?;
		// Always capture and persist the final diff, including when the model
		// returns a valid verification result with passed=false.
		section!("MANIFEST BUILD SNAPSHOT START");

		let workspace_after =
			WSSnapshot::capture(&workspace).context("capturing workspace after manifest Build")?;
		let changes = workspace_before.diff(&workspace_after);
		section!(&format!(
			"MANIFEST BUILD SNAPSHOT COMPLETE: files_changed={}",
			changes.file_count()
		));
		let status = if verification.passed {
			"Manifest Build completed"
		} else {
			"Manifest Build verification failed"
		};
		self.write(&changes.to_markdown(status))?;
		self.runtime.persist_progress(&format!(
			"Manifest Build {}; {} file(s) changed",
			if verification.passed {
				"verification passed"
			} else {
				"verification failed"
			},
			changes.file_count()
		))?;

		if !verification.passed {
			return Err(anyhow!(
				"Manifest Build verification failed: {}; findings: {}",
				verification.summary,
				verification.findings.join("; ")
			));
		}

		section!("MANIFEST BUILD STAGE COMPLETE");

		Ok(RunResult::Build)
	}

	async fn build_phase_assess(
		&mut self,
		log_dir: &Path,
		_intent: &str,
		_spec: &str,
		plan: &str,
	) -> Result<BuildManifest> {
		log::emit(E::BuildAssess);
		let phase_started = std::time::Instant::now();
		section!("MANIFEST BUILD PHASE START: Orient");
		let manifest = match self.runtime.system.runtime.assess(plan).await {
			Ok(manifest) => manifest,
			Err(error) => {
				section_info!(
					"MANIFEST BUILD PHASE FAILED",
					metrics: [
						"phase" => "Orient",
						"elapsed" => format!("{:?}", phase_started.elapsed()),
					],
					"error" => format!("{error:#}"),
				);

				return Err(error).context("Manifest Build Orient phase failed");
			}
		};
		let report = serde_json::to_string_pretty(&manifest).context("serializing Build manifest")?;
		tokio::fs::write(log_dir.join("build-manifest-assess.json"), &report)
			.await
			.context("writing manifest assessment report")?;
		section!(&format!(
			"MANIFEST BUILD PHASE COMPLETE: Orient elapsed={:?}",
			phase_started.elapsed()
		));
		Ok(manifest)
	}
	async fn build_phase_attest(
		&mut self,
		log_dir: &std::path::Path,
		workspace: &std::path::Path,
		workspace_before: &WSSnapshot,
		intent: &str,
		spec: &str,
		plan: &str,
		manifest: &BuildManifest,
		execution: &BuildExecutionReport,
	) -> Result<BuildVerification> {
		log::emit(E::BuildVerify);

		let phase_started = std::time::Instant::now();
		section!("MANIFEST BUILD PHASE START: Verify");

		let verification = match self
			.runtime
			.system
			.runtime
			.verify(manifest, execution, intent, spec, plan)
			.await
		{
			Ok(verification) => verification,
			Err(error) => {
				section_info!(
					"MANIFEST BUILD PHASE FAILED",
					metrics: [
						"phase" => "Verify",
						"elapsed" => format!("{:?}", phase_started.elapsed()),
					],
					"error" => format!("{error:#}"),
				);

				// Preserve changes if verification cannot complete.
				match WSSnapshot::capture(workspace) {
					Ok(snapshot) => {
						let changes = workspace_before.diff(&snapshot);

						section_info!(
							"MANIFEST BUILD PARTIAL WORK",
							metrics: [
								"phase" => "Verify",
								"files changed" => changes.file_count(),
							],
						);

						let _ = self.write(&changes.to_markdown("Manifest Build failed during Verify"));
					}
					Err(snapshot_error) => {
						section_info!(
							"MANIFEST BUILD SNAPSHOT FAILED",
							metrics: [
								"phase" => "Verify",
							],
							"error" => format!("{snapshot_error:#}"),
						);
					}
				}

				return Err(error).context("Manifest Build Verify phase failed");
			}
		};

		let report = serde_json::to_string_pretty(&verification)
			.context("serializing Build verification report")?;

		tokio::fs::write(log_dir.join("build-manifest-verify.json"), &report)
			.await
			.context("writing manifest verification report")?;

		section_info!(
			"MANIFEST BUILD VERIFICATION",
			metrics: [
				"passed" => verification.passed,
				"findings" => verification.findings.len(),
				"elapsed" => format!("{:?}", phase_started.elapsed()),
			],
			"summary" => &verification.summary,
		);

		section!(&format!(
			"MANIFEST BUILD PHASE COMPLETE: Verify elapsed={:?}",
			phase_started.elapsed()
		));

		Ok(verification)
	}
	async fn build_phase_execute(
		&mut self,
		log_dir: &std::path::Path,
		workspace: &std::path::Path,
		workspace_before: &WSSnapshot,
		intent: &str,
		spec: &str,
		plan: &str,
		manifest: &BuildManifest,
	) -> Result<BuildExecutionReport> {
		log::emit(E::BuildExecute);

		let phase_started = std::time::Instant::now();
		section!("MANIFEST BUILD PHASE START: Act");

		let execution = match self
			.runtime
			.system
			.runtime
			.execute(manifest, intent, spec, plan)
			.await
		{
			Ok(execution) => execution,
			Err(error) => {
				section_info!(
					"MANIFEST BUILD PHASE FAILED",
					metrics: [
						"phase" => "Act",
						"elapsed" => format!("{:?}", phase_started.elapsed()),
					],
					"error" => format!("{error:#}"),
				);

				// Preserve the workspace diff if execution partially changed files.
				match WSSnapshot::capture(workspace) {
					Ok(snapshot) => {
						let changes = workspace_before.diff(&snapshot);

						section_info!(
							"MANIFEST BUILD PARTIAL WORK",
							metrics: [
								"phase" => "Act",
								"files changed" => changes.file_count(),
							],
						);

						let _ = self.write(&changes.to_markdown("Manifest Build failed during Act"));
					}
					Err(snapshot_error) => {
						section_info!(
							"MANIFEST BUILD SNAPSHOT FAILED",
							metrics: [
								"phase" => "Act",
							],
							"error" => format!("{snapshot_error:#}"),
						);
					}
				}

				return Err(error).context("Manifest Build Act phase failed");
			}
		};

		let report =
			serde_json::to_string_pretty(&execution).context("serializing Build execution report")?;

		tokio::fs::write(log_dir.join("build-manifest-execute.json"), &report)
			.await
			.context("writing manifest execution report")?;

		section!(&format!(
			"MANIFEST BUILD PHASE COMPLETE: Act elapsed={:?}",
			phase_started.elapsed()
		));

		Ok(execution)
	}
	async fn build_phase_prepare(&mut self) -> Result<(PathBuf, String, String, String)> {
		let log_dir = self.stage_dir();
		if self.pipeline.stage != Stage::Build {
			return Err(anyhow!(
				"cannot execute Build stage while pipeline is at {:?}",
				self.pipeline.stage
			));
		}
		self.runtime.persist_progress("Build started")?;
		// Load and validate the approved SDLC artifacts once.
		let intent_path = log_dir.join("intent.md");
		let spec_path = log_dir.join("spec.md");
		let plan_path = log_dir.join("plan.md");

		let intent = tokio::fs::read_to_string(&intent_path)
			.await
			.context("reading intent.md")?;
		let spec = tokio::fs::read_to_string(&spec_path)
			.await
			.context("reading spec.md")?;
		let plan = tokio::fs::read_to_string(&plan_path)
			.await
			.context("reading plan.md")?;

		if intent.trim().is_empty() {
			return Err(anyhow::anyhow!("Intent artifact is empty"));
		}
		if spec.trim().is_empty() {
			return Err(anyhow::anyhow!("Spec artifact is empty"));
		}
		if plan.trim().is_empty() {
			return Err(anyhow::anyhow!("Plan artifact is empty"));
		}

		self.runtime.system.add_file(intent_path);
		self.runtime.system.add_file(spec_path);
		self.runtime.system.add_file(plan_path);
		Ok((log_dir, intent, spec, plan))
	}

	async fn on_qa(&mut self, _input: StageInput) -> Result<RunResult> {
		let verification = self.run_qa_checks(self.pipeline.stage).await?;
		self.runtime.persist_progress(&format!(
			"Verification completed: passed={}",
			verification.passed
		))?;
		Ok(RunResult::Verification(verification))
	}

	async fn route_execution(
		&mut self,
		stage: Stage,
		attempt: Attempt,
		input: StageInput,
	) -> Result<Execution> {
		let time_started = Utc::now();
		tracing::info!(?stage, "route_execution: dispatching");
		let result = match stage {
			Stage::Intent => self.on_intent(input).await?,
			Stage::Spec => self.on_spec(input).await?,
			Stage::Plan => self.on_plan(input).await?,
			Stage::Build => match self.on_build(input).await {
				Ok(result) => result,
				Err(error) => {
					eprintln!("Build stage failed: {error:#}");
					return Err(error);
				}
			},
			Stage::QA => self.on_qa(input).await?,
			Stage::Complete => RunResult::Complete,
			Stage::Finalize => RunResult::Finalize,
			Stage::Deploy | Stage::Maintain => {
				return Err(anyhow!("stage {stage:?} not implemented"));
			}
			_ => {
				todo!("run_stage")
			}
		};

		Ok(Execution {
			stage,
			attempt,
			time_started,
			time_completed: Utc::now(),
			result,
		})
	}
	async fn run_stage_inner(&mut self, _stage: Stage) -> Result<StageOutcome> {
		// Move the ORIGINAL implementation of run_stage here.
		todo!("move existing implementation here")
	}
	async fn run_qa_checks(&mut self, stage: Stage) -> Result<Verification> {
		tracing::info!("run_qa_checks");
		let checks = self.runtime.run_checks(stage).await?;
		let evaluations = self.evaluate_checks(&checks);
		let passed = self.pipeline.is_passing(&checks, &evaluations);
		Ok(Verification {
			passed,
			checks,
			evaluations,
		})
	}
	async fn _stage_deploy(&mut self) -> Result<()> {
		todo!("stage_deploy")
	}
	async fn _stage_maintain(&mut self) -> Result<()> {
		todo!("stage_maintain")
	}
	async fn _stage_complete(&mut self) -> Result<RunControl> {
		todo!("stage_complete")
	}
	async fn wait_for_intervention(
		&mut self,
		stage: Stage,
		attempt: Attempt,
		reason: String,
		input_rx: &mut UnboundedReceiver<SdlcInput>,
	) -> Result<Intervention> {
		self
			.pipeline
			.wait_for_intervention(stage, attempt, reason, input_rx)
			.await
	}
	pub fn initialize(pipeline: Pipeline, runtime: _Runtime<C>) -> PipelineState<C> {
		PipelineState { pipeline, runtime }
	}
	/// Save the current pipeline state and stage checkpoint.
	pub async fn checkpoint(&mut self) -> Result<()> {
		todo!("checkpoint")
	}
	/// Evaluate an existing artifact without rerunning the stage.
	pub async fn evaluate_stage(&mut self, _stage: Stage) -> Result<Evaluation> {
		todo!("evaluate_stage")
	}

	pub async fn execute_attempt(&mut self, stage: Stage) -> Result<AttemptOutcome> {
		self
			.execute_attempt_inner(stage)
			.await
			.context("_Runner::execute_attempt")
	}

	/// Prepare a stage without executing it.
	pub async fn prepare_stage(&mut self, _stage: Stage) -> Result<()> {
		todo!("prepare_stage")
	}
	/// Resume execution from the saved state.
	pub async fn resume(&mut self) -> Result<PipelineOutcome> {
		todo!("resume")
	}
	/// "What happens next?"
	/// Orchestrates the SDLC state machine:
	/// 1. run/execute stage -> Given the current stage, do business logic
	/// 2. persist outcome -> update the execution results, logs, state, etc
	/// 3. handle_outcome -> given the outcome decide what to do next.
	/// 4. apply -> given an outcome of execution handle business logic of that decision with run control.
	/// 5. control -> continue, retry, or exit. there is no other possibility.
	/// Run the entire pipeline from its current state.
	pub async fn run(
		&mut self,
		input_rx: &mut UnboundedReceiver<SdlcInput>,
	) -> Result<PipelineOutcome> {
		tracing::info!("Context: Ctx> _Runner");
		let mut pending_input = None;
		let mut stage = self.pipeline.stage;
		let stage_outcomes: Vec<StageOutcome> = Vec::new();
		log::emit(&E::RunBegin);
		loop {
			let attempt = self.next_attempt(stage)?;
			section_info!(
				"run",
				metrics: [
				"stage" => stage,
				"attempt" => attempt,
				],
			);
			let outcome = self
				.execute(stage, attempt, &mut pending_input)
				.await
				.with_context(|| format!("run_stage({stage:?})"))?;
			self.runtime.persist_outcome(&outcome)?;
			let decision = self.handle_outcome(&outcome).await?;
			tracing::info!("handle_outcome");
			let control = self
				.apply(outcome, decision, input_rx, &mut pending_input)
				.await?;
			match control {
				RunControl::Continue => {
					// tracing::info!("continue");
					stage = self.pipeline.stage;
					// Every new stage starts at attempt 1.
					self.pipeline.attempt_reset();
					// tracing::info!(
					// 	stage = ?stage,
					// 	attempt = self.pipeline.stage_attempt(),
					// 	">>> Runner continuing."
					// );
				}
				RunControl::RetryStage => {
					tracing::info!("RetryStage");
					continue;
				}
				RunControl::Exit(status) => {
					tracing::info!(?status, "Pipeline exiting");

					return Ok(PipelineOutcome {
						status,
						stages: stage_outcomes,
					});
				}
			}
		}
	}

	pub async fn run_simulated(
		&mut self,
		_input_rx: &mut UnboundedReceiver<SdlcInput>,
	) -> Result<()> {
		log::emit(E::RunBegin);
		for (index, step) in Step::ALL.iter().enumerate() {
			if *step == Step::Complete {
				break;
			}
			let Some(stage) = step.stage() else {
				continue;
			};
			let attempt = self.next_attempt(stage)?;
			log::emit(E::StageStarted { stage, attempt });
			log::emit(E::Activity {
				stage,
				attempt,
				message: format!("Dry Run · step {}/{}", index + 1, Step::ALL.len()),
			});

			self.pipeline.phase = Phase::Executing;
			log::emit(E::PhaseChanged {
				stage,
				phase: self.pipeline.phase,
			});

			sleep(DEMO_EXECUTION_TIME).await;
			log::emit(E::ExecutionComplete { stage });

			self.pipeline.phase = Phase::Evaluating;
			log::emit(E::PhaseChanged {
				stage,
				phase: self.pipeline.phase,
			});
			log::emit(E::EvaluationStarted { stage });

			sleep(DEMO_EVALUATION_TIME).await;
			log::emit(E::Evaluated {
				stage,
				score: 0.91,
				confidence: 0.94,
				passed: true,
			});

			let Some(next) = stage.next() else {
				break;
			};

			self.pipeline.transition(next)?;
			log_step_transition(stage, next)?;

			log::emit(E::StageTransitioned {
				from: stage,
				to: next,
			});
		}

		log::emit(E::Completed);
		sleep(DEMO_COMPLETE_DELAY).await;

		Ok(())
	}
	pub async fn run_stage(&mut self, stage: Stage) -> Result<StageOutcome> {
		self
			.run_stage_inner(stage)
			.await
			.context("_Runner::run_stage")
	}
	pub async fn run_stage_with(&mut self, _stage: Stage, _options: RunOptions) -> Result<Outcome> {
		todo!("run_stage_with")
	}
	pub async fn retry_stage(&mut self, _stage: Stage) -> Result<Outcome> {
		todo!("retry_stage")
	}
	pub async fn resume_from(
		&mut self,
		stage: Stage,
		input_rx: &mut UnboundedReceiver<SdlcInput>,
	) -> Result<PipelineOutcome> {
		tracing::info!(?stage, "resume_from pipeline");
		self.pipeline.stage = stage;
		self.pipeline.phase = Phase::Executing;
		self.pipeline.stage_attempt = 1;
		self.run(input_rx).await.context("_Runner::run")
	}
}
//
// impl<C> Ctx for _Runtime<C>
// where
// 	C: Clone + Send + Sync + 'static,
// {
// 	type State = State<C>;
//
// 	fn state() -> Self::State {
// 		todo!()
// 	}
// }

#[derive(Clone, Debug)]
struct State;
impl<C: Ctx> _Runtime<C> {
	fn bootstrap(ctx: &Kontex, goal: &str, resume: bool) -> anyhow::Result<PLSession> {
		if resume {
			Self::load_session(ctx)
		} else {
			Self::create_session(ctx, goal)
		}
	}
	fn create_session(ctx: &Kontex, intent: impl Into<String>) -> anyhow::Result<PLSession> {
		let intent = intent.into();
		let title = Pipeline::summarize_goal(&intent)?;
		let dir = Self::init_session(ctx, &title)?;
		Self::init_templates(ctx, &dir)?;
		let session = PLSession::new(title, intent, dir)?;
		ctx.session_save(&session)?;
		Ok(session)
	}
	fn init_session(kontex: &Kontex, _title: &str) -> anyhow::Result<PathBuf> {
		let dir = kontex.path(FW::Session)?;
		FS::ensure_dir(&dir)?;
		Ok(dir)
	}
	fn init_templates(kontex: &Kontex, dir: &Path) -> anyhow::Result<()> {
		let template_dir = kontex.path(FW::AiTemplates)?;
		for file in [
			SrcArtifact::Intent,
			SrcArtifact::Spec,
			SrcArtifact::Plan,
			SrcArtifact::Progress,
		] {
			let source = template_dir.join(file.name());
			let destination = file.path(dir);
			let contents = if FS::exists(&source) {
				FS::read(source)?
			} else {
				format!("# {}\n\n", file.name())
			};
			FS::write(destination, contents)?;
		}
		Ok(())
	}
	fn persist(&self) -> Result<()> {
		self.ctx.session_save(&self.session)
	}
	fn persist_progress(&mut self, message: &str) -> Result<()> {
		let session = self.session()?;
		let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
		let entry = format!("\n## {timestamp}\n\n{message}\n");
		SrcArtifact::Progress.append(&session.dir, entry)?;
		Ok(())
	}
	fn persist_evaluation(&mut self, evaluation: &QACheck, attempt: Attempt) -> Result<()> {
		let session = self.session()?;
		persist_evaluation(session, evaluation, attempt)?;
		self.persist()
	}
	fn persist_outcome(&mut self, outcome: &AttemptOutcome) -> Result<()> {
		let (stage, attempt, status, actor, time_started, time_completed, description, evaluation) =
			match outcome {
				AttemptOutcome::Progress {
					execution,
					evaluation,
				} => (
					execution.stage,
					execution.attempt,
					PLStatus::Completed,
					StageActor::Runner,
					execution.time_started,
					Some(execution.time_completed),
					format!("Stage Completed: {}", execution.stage),
					Some(evaluation.clone()),
				),
				AttemptOutcome::NeedsRevision {
					execution,
					evaluation,
				} => (
					execution.stage,
					execution.attempt,
					PLStatus::NeedsRevision,
					StageActor::Evaluator,
					execution.time_started,
					Some(execution.time_completed),
					format!("Needs Revision: {}", execution.stage),
					Some(evaluation.clone()),
				),
				AttemptOutcome::ExecutionFailed {
					stage,
					attempt,
					error,
				} => (
					*stage,
					*attempt,
					PLStatus::Failed,
					StageActor::Runner,
					Utc::now(),
					Some(Utc::now()),
					format!("{error:#}"),
					None,
				),
				AttemptOutcome::EvaluationFailed { execution, error } => (
					execution.stage,
					execution.attempt,
					PLStatus::EvaluationFailed,
					StageActor::Runner,
					execution.time_started,
					Some(execution.time_completed),
					error.to_string(),
					None,
				),
			};
		let record = StageRunRecord {
			stage,
			attempt,
			status,
			actor,
			time_started,
			time_completed,
			description: Some(description),
			evaluation,
		};
		self.push_stage_record(record)
	}
	fn persist_session(&mut self) -> Result<()> {
		let session = self.session()?;
		FS::save(SpecialFile::WriteCurrent.path()?, session)?;
		Ok(())
	}
	fn push_stage_record(&mut self, record: StageRunRecord) -> Result<()> {
		let session = self.session()?;
		session.stages.push(record);
		session.time_updated = Utc::now();
		self.persist()
	}
	fn session_read(&mut self, name: &str) -> Result<String> {
		read_from_session(name, self.session()?)
	}
	fn session(&mut self) -> Result<&mut PLSession> {
		Ok(&mut self.session)
	}
	fn load_session(ctx: &Kontex) -> anyhow::Result<PLSession> {
		let mut session = ctx
			.session_load::<PLSession>()?
			.ok_or_else(|| anyhow::anyhow!("No saved SDLC session is available to resume"))?;
		if session.workspace == session.dir || !session.workspace.is_dir() {
			session.workspace = session.cwd.clone();
			ctx.session_save(&session)?;
		}
		Ok(session)
	}
	pub fn view(&self, pipeline: &Pipeline) -> PLRuntimeSS {
		PLRuntimeSS {
			activity: self.activity.clone(),
			attempt: self.attempt,
			stage: pipeline.stage,
			time_started: self.time_started,
			stage_time_started: self.stage_time_started,
			phase: pipeline.phase,
			score: self.score,
			confidence: self.confidence,
			passed: self.passed,
			message: self.message.clone(),
			error: self.error.clone(),
			total_tokens: self.total_tokens,
			total_agent_calls: self.total_agent_calls,
			history: self.history.clone(),
			events: self.event_history.clone(),
		}
	}
	pub fn new(
		ctx: Kontex,
		qa: Evaluator,
		system: AgentSystem,
		generator: Box<dyn GeneratorTrait>,
		session: PLSession,
		runtime: Arc<NativeRuntime<C>>,
		event_bus: EventBus,
	) -> Self {
		Self {
			ctx,
			qa,
			system,
			generator,
			session,
			runtime,
			event_bus,
			activity: vec![],
			attempt: 0,
			confidence: None,
			error: None,
			event_history: vec![],
			history: vec![],
			message: None,
			passed: None,
			score: None,
			stage_time_started: Instant::now(),
			time_started: Instant::now(),
			total_agent_calls: 0,
			total_tokens: 0,
		}
	}

	pub async fn initialize(
		goal: &str,
		resume: bool,
		native_runtime: Arc<NativeRuntime<C>>,
		event_bus: EventBus,
	) -> anyhow::Result<(Self, Pipeline)> {
		let ctx = Kontex::new(profuse::resolved::App::Estate)?;
		let session = Self::bootstrap(&ctx, goal, resume)?;
		let qa = Evaluator {
			session: session.clone(),
			jev: TypeSafeClient::from_env()?,
		};

		let mut system = AgentSystem::new();
		system.cwd(session.workspace.clone());
		let generator = Box::new(LocalGenerator::new(system.runtime.clone(), DEFAULT_MODEL));
		let pipeline = Pipeline {
			stage: session.stage,
			phase: Phase::Starting,
			stage_attempt: 1,
		};
		let runtime = Self::new(
			ctx,
			qa,
			system,
			generator,
			session,
			native_runtime,
			event_bus,
		);
		Ok((runtime, pipeline))
	}
	pub async fn run(
		&mut self,
		pipeline: &mut Pipeline,
		input_rx: &mut UnboundedReceiver<SdlcInput>,
	) -> Result<PipelineOutcome> {
		let mut runner = _Runner {
			runtime: self,
			pipeline,
		};
		runner.run(input_rx).await.context("_Runner::run")
	}
	pub async fn run_checks(&self, stage: Stage) -> Result<Vec<CheckResult>> {
		if !matches!(stage, Stage::Build | Stage::QA) {
			return Ok(Vec::new());
		}
		let plan_path = self.session.dir.join("plan.md");
		let plan = std::fs::read_to_string(&plan_path)
			.with_context(|| format!("reading build plan at {}", plan_path.display()))?;
		let expected_outputs = p::expected_outputs_from_plan(&plan)?;
		let mut results = Vec::with_capacity(expected_outputs.len());
		let workspace = self.session.workspace.canonicalize().with_context(|| {
			format!(
				"canonicalizing project workspace {}",
				self.session.workspace.display()
			)
		})?;

		for required_file in &expected_outputs {
			let candidate = workspace.join(required_file);

			let (passed, output) = match candidate.canonicalize() {
				Ok(path) if path.starts_with(&workspace) && path.is_file() => {
					match std::fs::metadata(&path) {
						Ok(metadata) if metadata.len() > 0 => (true, "File exists and is non-empty".into()),
						Ok(_) => (false, "File is empty".into()),
						Err(error) => (false, error.to_string()),
					}
				}
				Ok(_) => (
					false,
					"Path is not a file inside the project workspace".into(),
				),
				Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
					(false, "Required file is missing".into())
				}
				Err(error) => (false, error.to_string()),
			};
			results.push(CheckResult {
				name: format!("required file {}", required_file.display()),
				passed,
				output: Some(output),
			});
		}
		Ok(results)
	}
}

pub struct Pipeline {
	pub stage: Stage,
	pub phase: Phase,
	pub stage_attempt: u32,
}

pub struct NativeRuntime<C> {
	_phantom: PhantomData<C>,
}

pub struct _Runtime<Context: Ctx> {
	// pub _phantom: PhantomData<Context>,
	pub ctx: Kontex,
	pub qa: Evaluator,
	pub system: AgentSystem,
	pub generator: Box<dyn GeneratorTrait>,
	pub session: PLSession,
	pub event_bus: EventBus,
	pub runtime: Arc<NativeRuntime<Context>>,

	// Current execution state
	pub attempt: u32,
	pub confidence: Option<f64>,
	pub score: Option<f64>,
	pub passed: Option<bool>,
	pub error: Option<String>,

	// Execution history and metrics
	pub activity: Vec<String>,
	pub event_history: Vec<E>,
	pub history: Vec<E>,
	pub message: Option<String>,
	pub stage_time_started: Instant,
	pub time_started: Instant,
	pub total_agent_calls: u32,
	pub total_tokens: u64,
}

pub struct _Runner<'a, Context: Ctx> {
	pub runtime: &'a mut _Runtime<Context>,
	pub pipeline: &'a mut Pipeline,
}

pub struct PipelineState<C: Ctx> {
	pub pipeline: Pipeline,
	pub runtime: _Runtime<C>,
}

pub struct RunOptions {
	pub artifact_policy: ArtifactPolicy,
	pub evaluate_after_run: bool,
	pub checkpoint_after_run: bool,
}

impl Default for RunOptions {
	fn default() -> Self {
		Self {
			artifact_policy: ArtifactPolicy::RequireValid,
			evaluate_after_run: true,
			checkpoint_after_run: true,
		}
	}
}
