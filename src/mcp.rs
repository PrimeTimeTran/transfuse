use crate::prelude::*;

use rmcp::{
	ErrorData as McpError, ServerHandler, handler::server::wrapper::Parameters, model::*, prompt,
	prompt_router, schemars, schemars::JsonSchema, tool, tool_handler, tool_router,
};

fn format_prompt_messages(msgs: Vec<rmcp::model::PromptMessage>) -> String {
	msgs
		.into_iter()
		.map(|m| match m.content {
			PromptMessageContent::Text { text } => text,
			other => format!("{other:?}"),
		})
		.collect::<Vec<_>>()
		.join("\n")
}

#[tool_router]
impl TempTool {
	#[tool(description = "Temporary tool to add two numbers")]
	pub fn add(&self, Parameters(AddVars { a, b }): Parameters<AddVars>) -> String {
		(a + b).to_string()
	}
}

#[tool_router]
impl Calculator {
	#[tool(description = "Add two numbers")]
	fn add(&self, Parameters(AddParams { a, b }): Parameters<AddParams>) -> String {
		(a + b).to_string()
	}
}

#[tool_handler(
	name = "calculator",
	version = "1.0.0",
	instructions = "A simple calculator"
)]
impl ServerHandler for Calculator {}

#[tool_router(server_handler)]
impl MyServer {
	#[tool(description = "Say hello")]
	pub async fn hello(&self) -> String {
		"Hello, world!".into()
	}
	pub fn get_info(&self) -> ServerInfo {
		let mut handler = ServerInfo::default();
		handler.capabilities = ServerCapabilities::builder()
			.enable_resources()
			.enable_tools()
			.build();
		handler
	}
	pub fn add(&self, Parameters(args): Parameters<AddVars>) -> String {
		(args.a + args.b).to_string()
	}
}

#[prompt_router]
impl MyServer {
	#[prompt(name = "greeting", description = "A simple greeting")]
	pub async fn greeting(&self) -> Vec<PromptMessage> {
		vec![PromptMessage::new_text(
			PromptMessageRole::User,
			"Hello! How can you help me today?",
		)]
	}
	#[prompt(name = "code_review", description = "Review code in a given language")]
	pub async fn code_review(
		&self,
		Parameters(args): Parameters<CodeReviewArgs>,
	) -> Result<GetPromptResult, McpError> {
		let focus = args
			.focus_areas
			.unwrap_or_else(|| vec!["correctness".into()]);

		let mut messages = vec![PromptMessage::new_text(
			PromptMessageRole::User,
			format!(
				"Please review my {} code. Focus on: {}",
				args.language,
				focus.join(", ")
			),
		)];

		let system_message = PromptMessage::new_text(
			PromptMessageRole::Assistant,
			"You are a helpful code reviewer. Provide constructive feedback.",
		);
		messages.insert(0, system_message);
		let mut result = GetPromptResult::default();
		result.messages = messages;
		Ok(result)
	}
}

impl McpClient {
	pub async fn hello(&self) -> Result<String> {
		Ok(self.server.hello().await)
	}
	pub async fn greeting(&self) -> Result<Vec<PromptMessage>> {
		Ok(self.server.greeting().await)
	}
	pub async fn code_review(&self, args: CodeReviewArgs) -> Result<String> {
		let res = self.server.code_review(Parameters(args)).await?;
		Ok(format_prompt_messages(res.messages))
	}
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddParams {
	a: i32,
	b: i32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddVars {
	pub a: i32,
	pub b: i32,
}

#[derive(Clone)]
struct Calculator;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CodeReviewArgs {
	#[schemars(description = "Programming language of the code")]
	pub language: String,
	#[schemars(description = "Focus areas for the review")]
	pub focus_areas: Option<Vec<String>>,
}

#[derive(Clone, Default, Debug)]
struct MyServer;

#[derive(Clone)]
struct TempTool;

#[derive(Default, Debug, Clone)]
pub struct McpClient {
	server: MyServer,
}
