use super::*;
use crate::prelude::*;

pub fn gen_intent(goal: &str) -> Result<String> {
	if goal.trim().is_empty() {
		return Err(anyhow!("cannot generate Intent prompt from empty goal"));
	}
	let prompt = AH_PROMPT_INTENT
		.replace("{template}", AH_TEMPLATE_INTENT)
		.replace("{goal}", goal);
	Ok(prompt)
}

pub fn gen_spec(intent: &str) -> Result<String> {
	if intent.trim().is_empty() {
		return Err(anyhow!("cannot generate Spec prompt from empty Intent"));
	}
	let prompt = AH_PROMPT_SPEC
		.replace("{template}", AH_TEMPLATE_SPEC)
		.replace("{intent}", intent);
	Ok(prompt)
}

pub fn gen_plan(intent: &str, spec: &str) -> Result<String> {
	if intent.trim().is_empty() {
		return Err(anyhow!("Intent artifact is empty"));
	}
	if spec.trim().is_empty() {
		return Err(anyhow!("Spec artifact is empty"));
	}
	let prompt = AH_PROMPT_PLAN
		.replace("{template}", AH_TEMPLATE_PLAN)
		.replace("{intent}", intent)
		.replace("{spec}", spec);
	Ok(prompt)
}

pub fn expected_outputs_from_plan(plan: &str) -> Result<Vec<PathBuf>> {
	let lines = plan.lines().collect::<Vec<_>>();

	let contract_index = lines
		.iter()
		.position(|line| {
			let line = line.trim_start();
			let level = line.chars().take_while(|&c| c == '#').count();

			if !(1..=6).contains(&level) || !line[level..].starts_with(' ') {
				return false;
			}

			let heading = line[level..].trim();

			// Accept headings such as:
			// ### 6.2 Machine-Readable Build Contract
			let mut parts = heading.splitn(2, char::is_whitespace);
			let first = parts.next().unwrap_or("");
			let rest = parts.next().unwrap_or("").trim();

			let title = if !first.is_empty()
				&& first
					.trim_end_matches('.')
					.chars()
					.all(|c| c.is_ascii_digit() || c == '.')
			{
				rest
			} else {
				heading
			};

			matches!(
				title,
				"Machine-Readable Output Inventory" | "Machine-Readable Build Contract"
			)
		})
		.ok_or_else(|| anyhow!("Plan is missing the Machine-Readable Output Inventory section"))?;

	let contract = lines[contract_index + 1..].join("\n");

	// The template may contain explanatory prose before the JSON fence.
	let _json = contract
		.find("```json")
		.map(|index| &contract[index + "```json".len()..])
		.and_then(|rest| rest.split_once("```").map(|(json, _)| json.trim()))
		.ok_or_else(|| anyhow!("Output Inventory must contain a fenced JSON object"))?;

	// Limit extraction to this section, stopping at the next H2 heading.
	let section_end = lines[contract_index + 1..]
		.iter()
		.position(|line| line.trim_start().starts_with("## "))
		.map(|offset| contract_index + 1 + offset)
		.unwrap_or(lines.len());

	let contract = lines[contract_index + 1..section_end].join("\n");
	// Find a JSON fence anywhere within the section, allowing explanatory text first.
	let json = contract
		.split_once("```json")
		.and_then(|(_, rest)| rest.split_once("```").map(|(json, _)| json.trim()))
		.ok_or_else(|| anyhow!("Output Inventory must contain a fenced JSON object"))?;

	#[derive(serde::Deserialize)]
	struct OutputInventory {
		#[serde(default, alias = "required_files")]
		expected_outputs: Vec<PathBuf>,
	}
	let inventory: OutputInventory =
		serde_json::from_str(json).context("parsing Plan output inventory")?;

	let mut expected_outputs = inventory.expected_outputs;

	// Include test locations declared elsewhere in the Plan.
	for line in &lines {
		let Some((_, location)) = line.split_once("Test Location:") else {
			continue;
		};
		let Some((_, location)) = location.split_once('`') else {
			continue;
		};
		let Some((location, _)) = location.split_once('`') else {
			continue;
		};

		let path = PathBuf::from(location.trim());
		if !expected_outputs.contains(&path) {
			expected_outputs.push(path);
		}
	}
	// Reject unsafe or invalid relative paths.
	for path in &expected_outputs {
		if path.as_os_str().is_empty()
			|| path.is_absolute()
			|| path
				.components()
				.any(|component| matches!(component, std::path::Component::ParentDir))
		{
			return Err(anyhow!(
				"Plan contains an invalid output path: {}",
				path.display()
			));
		}
	}
	Ok(expected_outputs)
}
// pub fn plan_gen(intent: &str, spec: &str) -> String {
// 	format!(
// 		r#"
// You are creating an implementation plan for an SDLC system.
//
// The user's intent is authoritative.
//
// ## Intent
//
// {intent}
//
// ## Specification
//
// {spec}
//
// ## Instructions
//
// Create a concrete implementation plan.
//
// The plan must:
// - identify the implementation work required
// - break the work into ordered steps
// - identify files/components likely to change
// - identify dependencies between steps
// - identify how each requirement will be verified
// - avoid inventing requirements not present in the intent or specification
//
// Return only the contents of `plan.md`.
// "#,
// 	)
// }
pub fn revise_spec(intent: &str, current_spec: &str, evaluation: &QACheck) -> Result<String> {
	Ok(format!(
		r#"Revise the existing specification so that it meets the requirements of the evaluator.

## Intent

{intent}

## Current Specification

{current_spec}

## Evaluation

Score: {score}
Confidence: {confidence}
Passed: {passed}

{evaluations}

## Instructions

Revise the specification to address the evaluator's failures.

Preserve correct information from the existing specification.

Do not merely describe the changes.
Produce the complete revised specification.

The output will replace spec.md directly.
"#,
		score = evaluation.score,
		confidence = evaluation.confidence,
		passed = evaluation.passed,
		evaluations = evaluation
			.evaluations
			.iter()
			.map(|e| {
				format!(
					"- {}: passed={}, score={}, confidence={}\n  {}",
					e.name, e.passed, e.score, e.confidence, e.explanation
				)
			})
			.collect::<Vec<_>>()
			.join("\n"),
	))
}
// pub fn tests_gen(intent: &str, spec: &str, plan: &str) -> String {
// 	format!(
// 		r#"
// 			You are designing the verification plan for an SDLC task.
//
// 			The user's intent is authoritative.
//
// 			## Intent
//
// 			{intent}
//
// 			## Specification
//
// 			{spec}
//
// 			## Implementation Plan
//
// 			{plan}
//
// 			## Instructions
//
// 			Create `tests.md`.
//
// 			Every requirement in the specification must have at least
// 			one corresponding verification test.
//
// 			Tests should distinguish between:
//
// 			1. deterministic checks
// 				- cargo test
// 				- cargo check
// 				- cargo clippy
// 				- cargo fmt
// 				- application-specific commands
//
// 			2. behavioral tests
// 				- unit tests
// 				- integration tests
// 				- end-to-end tests
//
// 			3. semantic verification
// 				- requirements that cannot be established purely through
// 					deterministic commands and should later be evaluated by JEV
//
// 			Each test must be concrete enough that another agent can implement
// 			or execute it.
//
// 			Use this format:
//
// 			# Tests
//
// 			## Requirement: <requirement>
//
// 			- [ ] <test>
//
// 			Do not mark any test as complete.
//
// 			Return only the contents of `tests.md`.
// 		"#,
// 	)
// }

#[cfg(test)]
mod completion_contract_tests {
	use super::*;

	#[test]
	fn parses_plan_output_inventory_without_commands() {
		let plan = r#"## 9. Machine-Readable Output Inventory
```json
{
  "expected_outputs": ["hello-world.cjs", "tests/hello-world.test.js"]
}
```
"#;
		let outputs = expected_outputs_from_plan(plan).unwrap();
		assert_eq!(outputs.len(), 2);
	}

	#[test]
	fn rejects_output_paths_outside_the_workspace() {
		let plan = r#"## Machine-Readable Output Inventory
```json
{"expected_outputs":["../outside"]}
```
"#;
		assert!(expected_outputs_from_plan(plan).is_err());
	}

	#[test]
	fn includes_test_locations_in_expected_outputs() {
		let plan = r#"## 5.1 Unit Tests
- [ ] **Test Location:** `test/hello-world.test.js`

## 9. Machine-Readable Output Inventory
```json
{"expected_outputs":["hello-world.cjs"]}
```
"#;
		let outputs = expected_outputs_from_plan(plan).unwrap();
		assert!(outputs.contains(&PathBuf::from("test/hello-world.test.js")));
	}
}
