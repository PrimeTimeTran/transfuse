use super::*;
use crate::prelude::*;

impl Clone for Box<dyn GeneratorTrait> {
	fn clone(&self) -> Self {
		self.as_ref().clone_box()
	}
}

#[async_trait::async_trait]
pub trait GeneratorTrait: Send + Sync + Debug {
	fn clone_box(&self) -> Box<dyn GeneratorTrait>;

	async fn generate(&self, prompt: &str) -> Result<String>;
	async fn run_agent(&self, prompt: &str) -> Result<String>;
	async fn with_session(&mut self, session: &PLSession, prompt: String) -> Result<TaskResult>;
}

#[async_trait::async_trait]
pub trait RunnerTrait {
	type Context;
	type Output;
	async fn run(&mut self, ctx: &mut Self::Context) -> Result<Self::Output>;
}
pub trait StageTrait {
	fn checkpoint(&self, _ctx: &mut Context) -> Result<()> {
		Ok(())
	}
	fn cleanup(&self, _ctx: &mut Context) -> Result<()> {
		Ok(())
	}
	fn description(&self) -> &'static str {
		""
	}
	fn evaluate(&self, _ctx: &Context, _result: &RunResult) -> Result<Option<Evaluation>> {
		Ok(None)
	}
	fn name(&self) -> &'static str;
	fn prepare(&self, _ctx: &mut Context) -> Result<()> {
		Ok(())
	}
	fn resume(&self, _ctx: &mut Context) -> Result<()> {
		Ok(())
	}
	fn run(&self, ctx: &mut Context) -> Result<RunResult>;
	fn transition(
		&self,
		_ctx: &Context,
		_result: &RunResult,
		_evaluation: Option<&Evaluation>,
	) -> Result<Transition> {
		Ok(Transition::Next)
	}
	fn validate(&self, _ctx: &mut Context) -> Result<()> {
		Ok(())
	}

	fn on_failure(&self, _ctx: &mut Context) -> Result<()> {
		Ok(())
	}
}

pub trait TextModel {
	async fn generate(&self, prompt: &str) -> Result<String>;
}

