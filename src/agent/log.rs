use super::*;
use crate::prelude::*;

fn append_line(path: &str, message: &str) -> anyhow::Result<()> {
	fs::create_dir_all(LOG_DIR)?;
	let mut file = OpenOptions::new()
		.create(true)
		.append(true)
		.open(Path::new(path))?;
	writeln!(file, "{message}")?;
	file.flush()?;
	Ok(())
}

fn fmt_event_append(message: &str) -> anyhow::Result<()> {
	let timestamp = chrono::Local::now().to_rfc3339();
	let message = message
		.replace('\\', "\\\\")
		.replace('\r', "\\r")
		.replace('\n', "\\n");
	append_line(AGENT_EVENT_LOG, &format!("[{timestamp}] {message}"))
}
pub fn event(event: impl std::fmt::Debug) -> anyhow::Result<()> {
	fmt_event_append(&format!("{event:?}"))
}
pub fn emit(event: impl std::fmt::Debug) {
	let _result = fmt_event_append(&format!("{event:?}"));
}
pub fn log(event: &impl std::fmt::Debug) -> anyhow::Result<()> {
	fmt_event_append(&format!("{event:?}"))
}
pub fn log_it(message: impl AsRef<str>) -> anyhow::Result<()> {
	let timestamp = chrono::Local::now().to_rfc3339();
	let message = truncate(message.as_ref(), 50);
	append_line(AGENT_LOG, &format!("[{timestamp}] {message}"))
}

pub fn truncate(prompt: &str, max_chars: usize) -> String {
	let prompt = prompt.replace('\n', "\\n").replace('\r', "\\r");
	if prompt.chars().count() <= max_chars {
		return prompt;
	}
	let truncated: String = prompt.chars().take(max_chars).collect();
	format!("{truncated}...")
}
