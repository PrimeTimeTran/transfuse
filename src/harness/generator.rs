use super::{PLSession, sdlc_trait as t, *};
use crate::prelude::*;

#[async_trait]
impl t::GeneratorTrait for ApiGenerator {
	async fn generate(&self, _prompt: &str) -> Result<String> {
		todo!("API generate")
	}
	async fn run_agent(&self, _prompt: &str) -> Result<String> {
		todo!("API generate")
	}
	async fn with_session(&mut self, _session: &PLSession, _prompt: String) -> Result<TaskResult> {
		todo!("with_session")
	}
	fn clone_box(&self) -> Box<dyn GeneratorTrait> {
		Box::new(self.clone())
	}
}
#[async_trait]
impl t::GeneratorTrait for LocalGenerator {
	async fn generate(&self, prompt: &str) -> Result<String> {
		let request = serde_json::json!({
			"model": self.model,
			"prompt": prompt,
			"stream": false,
			"options": {
				"num_ctx": ctx_window_size(),
			},
		});
		std::fs::write(
			ah_log_file("ollama-request.json"),
			serde_json::to_string_pretty(&request)?,
		)?;
		let response = reqwest::Client::new()
			.post(AGENT_GEN_URL)
			.json(&request)
			.send()
			.await?
			.error_for_status()?;
		let body = response.text().await?;
		std::fs::write(ah_log_file("ollama-response.json"), &body)?;
		let response: OllamaResponse =
			serde_json::from_str(&body).context("invalid Ollama response")?;
		let artifact = response.response.trim();
		if artifact.is_empty() {
			return Err(anyhow!(
				"Ollama returned an empty artifact \
				 (model={}, done_reason={:?})",
				self.model,
				response.done_reason,
			));
		}
		Ok(artifact.to_string())
	}
	// cargo run --bin sanity-tests -- --native src/bin/sanity-tests/tools-intern
	// cargo run --bin sanity-tests -- --native src/bin/sanity-tests/tools-host-env
	async fn run_agent(&self, prompt: &str) -> Result<String> {
		let task = AgentTask::new(prompt.to_string());
		let result = self
			.runtime
			.run_agent(task)
			// .run_agent_with_sdlc(task, Path::new("/Users/future/kb/project/crates/estate/log/"))
			.await?;
		Ok(
			result
				.chat
				.or(result.summary)
				.unwrap_or_else(|| "Agent completed".to_string()),
		)
	}
	async fn with_session(&mut self, session: &PLSession, prompt: String) -> Result<TaskResult> {
		let task = AgentTask::new(prompt.to_string());
		let result = self.runtime.from_session(task, session).await?;
		Ok(result)
	}
	fn clone_box(&self) -> Box<dyn GeneratorTrait> {
		Box::new(self.clone())
	}
}
impl LocalGenerator {
	pub fn new(runtime: AgentRuntime, model: impl Into<String>) -> Self {
		Self {
			runtime,
			model: model.into(),
		}
	}
}
