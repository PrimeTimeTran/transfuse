use super::*;
use crate::prelude::*;

impl Evaluator {
	pub async fn evaluate(&self, execution: &Execution) -> Result<QACheck> {
		let ctx = CtxEvaluation::load(&self.session, execution.stage)?;
		match execution.stage {
			Stage::Intent => self.intent(&ctx).await,
			Stage::Spec => self.spec(&ctx).await,
			Stage::Plan => self.plan(&ctx).await,
			Stage::Build => self.build(&ctx).await,
			Stage::QA => self.qa(&ctx).await,
			stage => Err(anyhow::anyhow!(
				"stage {stage:?} does not support evaluation"
			)),
		}
	}
	pub async fn goal(&self, goal: &str) -> Result<QACheck> {
		let time_started = Utc::now();
		let state = format!(
			"## User Goal\n\n{goal}\n\n\
			## Evaluation Context\n\n\
			Evaluate this as a software-development goal. \
			The goal should be useful as the starting point for an \
			SDLC pipeline. Judge the goal itself, not how well an \
			implementation could compensate for missing information."
		);
		let response = self
			.jev
			.system_one(
				state,
				[
					(
						"smart",
						Question::from(Score::new(
							"How well does this goal satisfy the SMART criteria \
							 for a software-development task?",
							[
								"Unusable: the goal is unclear, unbounded, or not actionable",
								"Weak: several SMART dimensions are substantially missing",
								"Usable: most SMART dimensions are present, but important ambiguity remains",
								"Strong: the goal is specific, measurable, attainable, realistic, and timely",
								"Excellent: the goal is precise, bounded, measurable, realistic, \
								 and provides a strong basis for implementation and verification",
							],
						)),
					),
					(
						"specific",
						Question::from(Score::new(
							"How specific is this software goal?",
							[
								"Unusable: no concrete software outcome is identifiable",
								"Weak: a general desire is expressed but the target outcome is ambiguous",
								"Usable: the primary software outcome is identifiable with some ambiguity",
								"Strong: the software outcome, scope, and relevant target are clearly defined",
								"Excellent: the desired software outcome and scope are precise and unambiguous",
							],
						)),
					),
					(
						"measurable",
						Question::from(Score::new(
							"How measurable is this software goal?",
							[
								"Unusable: there is no way to determine whether the goal was achieved",
								"Weak: success is mostly subjective or requires substantial invention",
								"Usable: some observable success criteria can be inferred",
								"Strong: success can be established through concrete observable criteria",
								"Excellent: the goal contains explicit, verifiable measures of success",
							],
						)),
					),
					(
						"attainable",
						Question::from(Score::new(
							"How attainable is this goal as a software-development task?",
							[
								"Unusable: the requested outcome is technically incoherent or impossible to act on",
								"Weak: substantial feasibility or dependency uncertainty exists",
								"Usable: the task appears feasible with some assumptions",
								"Strong: the requested outcome is plausibly achievable with ordinary software-development work",
								"Excellent: the goal is clearly achievable within the stated scope and constraints",
							],
						)),
					),
					(
						"realistic",
						Question::from(Score::new(
							"How realistic is this goal given the stated software context, \
							 constraints, dependencies, and expected scope?",
							[
								"Unusable: the goal conflicts with its context or contains unrealistic assumptions",
								"Weak: important assumptions or scope problems make the goal questionable",
								"Usable: the goal is broadly realistic but some assumptions need clarification",
								"Strong: the goal fits its context, constraints, and expected implementation scope",
								"Excellent: the goal is well-bounded and realistic with no major unstated assumptions",
							],
						)),
					),
					(
						"timely",
						Question::from(Score::new(
							"How well does this goal define a useful timeframe or completion boundary \
							 for the software task?",
							[
								"Unusable: there is no meaningful completion boundary",
								"Weak: completion timing or boundaries are substantially unclear",
								"Usable: the scope provides an implicit or approximate completion boundary",
								"Strong: the goal provides a clear completion boundary or useful timeframe",
								"Excellent: the goal contains a precise, verifiable timeframe, deadline, \
								 milestone, or explicit completion boundary",
							],
						)),
					),
					(
						"meets_bar",
						Question::from(Noul::new(
							"Is this goal sufficiently clear, bounded, and software-actionable \
							 to begin an SDLC process without an implementation agent inventing \
							 major requirements?",
						)),
					),
				],
			)
			.await?;
		let smart = response
			.score("smart")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no SMART score"))?;
		let dimensions = [
			("specific", "specificity"),
			("measurable", "measurability"),
			("attainable", "attainability"),
			("realistic", "realism"),
			("timely", "timeliness"),
		];
		let mut evaluations = Vec::with_capacity(7);
		evaluations.push(Metric {
			name: "smart".into(),
			passed: smart.score >= DEFAULT_NAUL_BAR,
			score: smart.score,
			confidence: smart.confidence,
			explanation: String::new(),
		});
		for (key, name) in dimensions {
			let result = response
				.score(key)
				.ok_or_else(|| anyhow::anyhow!("JEV returned no {name} score"))?;
			evaluations.push(Metric::quality(result));
		}
		let meets_bar = response
			.noul("meets_bar")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no goal decision"))?;
		evaluations.push(Metric::meets_bar(meets_bar));
		Ok(QACheck::new(
			Stage::Intent,
			time_started,
			smart.score,
			smart.confidence,
			meets_bar.noul,
			evaluations,
		))
	}
	pub async fn intent(&self, ctx: &CtxEvaluation) -> Result<QACheck> {
		let time_started = Utc::now();
		let intent = ctx.get(SrcArtifact::Intent)?;
		// tracing::info!("Evaluator intent {:?}", intent);
		let response = self
			.jev
			.system_one(
				intent,
				[
					(
						"quality",
						Question::from(Score::new(
							"How well does this intent define a concrete software task?",
							[
								"Unusable: the desired outcome is unclear or not actionable",
								"Weak: some intent is present, but major ambiguity remains",
								"Usable: the intended outcome is understandable with some ambiguity",
								"Strong: the desired outcome is concrete and actionable",
								"Excellent: the desired outcome is precise, bounded, and directly actionable",
							],
						)),
					),
					(
						"meets_bar",
						Question::from(Noul::new(
							"Does this intent provide a sufficiently clear and concrete \
                         desired outcome for an implementation agent to act on \
                         without inventing major requirements?",
						)),
					),
				],
			)
			.await?;
		let quality = response
			.score("quality")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no intent quality score"))?;
		let meets_bar = response
			.noul("meets_bar")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no intent decision"))?;
		Ok(QACheck::new(
			Stage::Intent,
			time_started,
			quality.score,
			quality.confidence,
			meets_bar.noul,
			vec![Metric::quality(quality), Metric::meets_bar(meets_bar)],
		))
	}
	pub async fn spec(&self, ctx: &CtxEvaluation) -> Result<QACheck> {
		let time_started = Utc::now();
		let intent = ctx.get(SrcArtifact::Intent)?;
		let spec = ctx.get(SrcArtifact::Spec)?;
		let state = format!(
			"## User Intent\n\n{intent}\n\n\
        ## Specification\n\n{spec}"
		);
		let response = self
			.jev
			.system_one(
				state,
				[
					(
						"quality",
						Question::from(Score::new(
							"How faithfully does the specification translate the intent \
                         into concrete, testable requirements?",
							[
								"Unusable: requirements are missing, contradictory, or unrelated",
								"Weak: substantial requirements are missing or invented",
								"Usable: the main intent is represented but some details are weak",
								"Strong: requirements are concrete, relevant, and testable",
								"Excellent: requirements comprehensively and precisely capture the intent \
                             without inventing unnecessary scope",
							],
						)),
					),
					(
						"meets_bar",
						Question::from(Noul::new(
							"Does the specification faithfully represent the user's intent \
                         and provide sufficiently concrete requirements for planning \
                         and verification?",
						)),
					),
				],
			)
			.await?;
		let quality = response
			.score("quality")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no spec quality score"))?;
		let meets_bar = response
			.noul("meets_bar")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no spec decision"))?;
		Ok(QACheck::new(
			Stage::Spec,
			time_started,
			quality.score,
			quality.confidence,
			meets_bar.noul,
			vec![Metric::quality(quality), Metric::meets_bar(meets_bar)],
		))
	}
	pub async fn plan(&self, ctx: &CtxEvaluation) -> Result<QACheck> {
		let time_started = Utc::now();
		let intent = ctx.get(SrcArtifact::Intent)?;
		let spec = ctx.get(SrcArtifact::Spec)?;
		let plan = ctx.get(SrcArtifact::Plan)?;
		let state = format!(
			"## User Intent\n\n{intent}\n\n\
         ## Specification\n\n{spec}\n\n\
         ## Implementation Plan\n\n{plan}\n\n\
         "
		);
		let response = self
			.jev
			.system_one(
				state,
				[
					(
						"quality",
						Question::from(Score::new(
							"How well does the implementation and test plan cover \
                         the specification?",
							[
								"Unusable: the plan does not provide a viable path to implementation",
								"Weak: major requirements or verification steps are uncovered",
								"Usable: the main implementation and verification work is covered",
								"Strong: requirements map clearly to implementation and verification steps",
								"Excellent: the plan is complete, ordered, dependency-aware, and provides \
                             explicit verification coverage for every requirement",
							],
						)),
					),
					(
						"meets_bar",
						Question::from(Noul::new(
							"Does the implementation plan provide a concrete path from the \
                         specification to implementation, while ensuring that every \
                         requirement has corresponding verification coverage?",
						)),
					),
				],
			)
			.await?;
		let quality = response
			.score("quality")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no plan quality score"))?;
		let meets_bar = response
			.noul("meets_bar")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no plan decision"))?;
		Ok(QACheck::new(
			Stage::Plan,
			time_started,
			quality.score,
			quality.confidence,
			meets_bar.noul,
			vec![Metric::quality(quality), Metric::meets_bar(meets_bar)],
		))
	}
	pub async fn build(&self, ctx: &CtxEvaluation) -> Result<QACheck> {
		let time_started = Utc::now();
		let intent = ctx.get(SrcArtifact::Intent)?;
		let spec = ctx.get(SrcArtifact::Spec)?;
		let plan = ctx.get(SrcArtifact::Plan)?;
		let implementation = std::fs::read_dir(&self.session.workspace)?
			.filter_map(|entry| entry.ok())
			.filter_map(|entry| {
				let path = entry.path();
				let name = path.file_name()?.to_string_lossy().into_owned();

				Some(if path.is_dir() {
					format!("[directory] {name}")
				} else {
					format!("[file] {name}")
				})
			})
			.collect::<Vec<_>>()
			.join("\n");
		let state = format!(
			"## User Intent\n\n{intent}\n\n\
			## Specification\n\n{spec}\n\n\
			## Implementation Plan\n\n{plan}\n\n\
			## Repository Artifacts\n\n{implementation}"
		);
		let response = self
			.jev
			.system_one(
				state,
				[
					(
						"quality",
						Question::from(Score::new(
							"How faithfully does the implemented work satisfy the \
						 specification and implementation plan?",
							[
								"Unusable: the implementation does not meaningfully address the task",
								"Weak: substantial requirements are missing or the implementation \
							 diverges from the plan",
								"Usable: the primary requirements appear implemented but some \
							 gaps or deviations remain",
								"Strong: the implementation closely follows the specification \
							 and provides the planned functionality",
								"Excellent: the implementation comprehensively satisfies the \
							 specification and plan with no significant unexplained gaps",
							],
						)),
					),
					(
						"meets_bar",
						Question::from(Noul::new(
							"Does the implementation appear to satisfy the specified requirements \
						 and provide the functionality described by the implementation plan?",
						)),
					),
				],
			)
			.await?;
		let quality = response
			.score("quality")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no build quality score"))?;
		let meets_bar = response
			.noul("meets_bar")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no build decision"))?;
		Ok(QACheck::new(
			Stage::Build,
			time_started,
			quality.score,
			quality.confidence,
			meets_bar.noul,
			vec![Metric::quality(quality), Metric::meets_bar(meets_bar)],
		))
	}
	pub async fn qa(&self, ctx: &CtxEvaluation) -> Result<QACheck> {
		let time_started = Utc::now();
		let intent = ctx.get(SrcArtifact::Intent)?;
		let spec = ctx.get(SrcArtifact::Spec)?;
		// let tests = ctx.get(SrcArtifact::Test)?;
		let evidence = ctx
			.get(SrcArtifact::QA)
			.unwrap_or("No verification evidence was recorded.");
		let state = format!(
			"## User Intent\n\n{intent}\n\n\
		 ## Specification\n\n{spec}\n\n\
		 ## Verification Evidence\n\n{evidence}"
		);
		let response = self
			.jev
			.system_one(
				state,
				[
					(
						"quality",
						Question::from(Score::new(
							"How strong is the verification evidence for establishing \
						 that the implementation satisfies the specification?",
							[
								"Unusable: there is no meaningful verification evidence",
								"Weak: some checks exist but important requirements are unverified",
								"Usable: the primary requirements have verification evidence but \
							 some gaps remain",
								"Strong: deterministic and behavioral evidence covers the \
							 requirements with only minor gaps",
								"Excellent: verification provides comprehensive, concrete evidence \
							 for every requirement and clearly establishes the intended behavior",
							],
						)),
					),
					(
						"meets_bar",
						Question::from(Noul::new(
							"Does the verification evidence provide sufficient evidence that \
						 every requirement in the specification has been satisfied?",
						)),
					),
				],
			)
			.await?;
		let quality = response
			.score("quality")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no verification quality score"))?;
		let meets_bar = response
			.noul("meets_bar")
			.ok_or_else(|| anyhow::anyhow!("JEV returned no verification decision"))?;
		Ok(QACheck::new(
			Stage::QA,
			time_started,
			quality.score,
			quality.confidence,
			meets_bar.noul,
			vec![Metric::quality(quality), Metric::meets_bar(meets_bar)],
		))
	}
}
