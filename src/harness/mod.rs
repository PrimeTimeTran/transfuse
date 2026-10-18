mod r#const;

#[path = "./enum.rs"]
mod sdlc_enum;
use profuse::prelude::Ctx;
pub use sdlc_enum::*;

mod evaluator;
pub mod r#fn;
mod generator;
mod r#impl;

mod outcome;
use outcome::*;

mod pipeline;
pub use pipeline::*;
mod prompt;
use prompt as p;

mod r#struct;

#[path = "./trait.rs"]
mod sdlc_trait;
pub use sdlc_trait::*;

mod stage;
mod ui;
mod workspace;

pub(crate) use r#const::*;
pub(crate) use r#fn::*;
pub(crate) use r#struct::{PLSession, *};

#[derive(Debug, Clone)]
pub struct HarnessCtx;

#[derive(Debug, Clone)]
pub struct HarnessState {
	pub count: usize,
}

impl Ctx for HarnessCtx {
	type State = HarnessState;

	fn state() -> Self::State {
		HarnessState { count: 0 }
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn build_steps_are_small_and_sequential() {
		let steps = build_steps();
		assert!(!steps.is_empty());
		assert!(steps[0].contains("plan.md"));
		assert!(steps[1].contains("Inspect"));
		assert!(steps.iter().any(|s| s.contains("tests")));
		assert!(steps.iter().any(|s| s.contains("failures")));
		assert!(steps.last().unwrap().contains("remaining work"));
	}

	#[test]
	fn build_step_prompt_contains_step_and_context() {
		let prompt = build_step_prompt(
			"Read plan.md and identify the files that must be created or modified.",
			1,
			10,
			"Plan says to create hello-world.js.",
			"CWD: /project\nFILES:\nhello-world.js",
		);

		assert!(prompt.contains("BUILD STEP 1/10"));
		assert!(prompt.contains("hello-world.js"));
		assert!(prompt.contains("Plan says"));
	}
}
