use super::*;
use std::fmt::Write;

pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const CYAN: &str = "\x1b[36m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";
pub const MAGENTA: &str = "\x1b[35m";
pub const BOLD: &str = "\x1b[1m";

pub fn fmt_section(title: &str) -> String {
	format!("\n{}\n", colorize(CYAN, &format!("--- {title} ---")),)
}

pub fn fmt_kv(k: &str, v: &str) -> String {
	format!("{} {}", colorize(BOLD, k), colorize(DIM, v),)
}

pub fn fmt_manifest(manifest: &BuildManifest) -> String {
	let mut output = fmt_section("EXPECTED FILE OPERATIONS");

	let mut create = 0;
	let mut update = 0;
	let mut delete = 0;

	let path_width = manifest
		.files
		.iter()
		.map(|file| file.path.display().to_string().chars().count())
		.max()
		.unwrap_or(4)
		.max(4);

	for file in &manifest.files {
		let operation = match file.operation {
			BuildOperation::Create => {
				create += 1;
				"CREATE"
			}
			BuildOperation::Update => {
				update += 1;
				"UPDATE"
			}
			BuildOperation::Delete => {
				delete += 1;
				"DELETE"
			}
		};

		let path = file.path.display().to_string();
		let path_padding = path_width.saturating_sub(path.chars().count());

		output.push_str(&format!(
			"{}  {}{}  {}\n",
			colorize(BOLD, operation),
			path,
			" ".repeat(path_padding),
			colorize(DIM, &file.reason),
		));
	}

	output.push('\n');
	output.push_str(&fmt_kv(
		"files",
		&format!(
			"{} total | {} create | {} update | {} delete",
			manifest.files.len(),
			create,
			update,
			delete,
		),
	));
	output.push('\n');

	output
}

pub fn fmt_output(_label: &str, content: &str, max_chars: usize, max_lines: usize) -> String {
	let total_chars = content.chars().count();
	let total_lines = content.lines().count();

	let mut preview = String::new();
	let mut truncated = false;

	for (index, line) in content.lines().enumerate() {
		if index >= max_lines {
			truncated = true;
			break;
		}

		if !preview.is_empty() {
			preview.push('\n');
		}

		preview.push_str(line);

		if preview.chars().count() > max_chars {
			truncated = true;
			break;
		}
	}

	if preview.chars().count() > max_chars {
		preview = preview.chars().take(max_chars).collect();
		truncated = true;
	}

	let preview_lines = preview.lines().count();

	let mut output = format!(
		"{} | {} lines | {} chars\n",
		colorize(DIM, if truncated { "TRUNCATED" } else { "FULL" }),
		total_lines,
		total_chars,
	);

	output.push_str(&format!("\n{}", colorize(DIM, &preview)));

	if truncated {
		output.push_str(&format!(
			"\n{}",
			colorize(
				DIM,
				&format!(
					"... output truncated (showing up to {max_chars} chars, {max_lines} lines; preview has {preview_lines} lines)"
				)
			)
		));
	}

	output
}

pub fn fmt_agent_metrics(
	history: usize,
	commands: usize,
	errors: usize,
	logs: usize,
	tokens: usize,
	files: usize,
) -> String {
	format!(
		"{} {} {} {} {} {}",
		colorize(CYAN, format!("history={history}")),
		colorize(GREEN, format!("cmds={commands}")),
		colorize(
			if errors > 0 { RED } else { DIM },
			format!("errors={errors}")
		),
		colorize(YELLOW, format!("logs={logs}")),
		colorize(MAGENTA, format!("tokens={tokens}")),
		colorize(CYAN, format!("files={files}")),
	)
}
pub fn fmt_changed_files(paths: &[std::path::PathBuf]) -> String {
	if paths.is_empty() {
		return "(no filesystem changes detected)".to_owned();
	}
	let mut paths = paths
		.iter()
		.map(|path| path.display().to_string())
		.collect::<Vec<_>>();
	paths.sort();
	paths.dedup();
	paths
		.into_iter()
		.map(|path| format!("- {path}"))
		.collect::<Vec<_>>()
		.join("\n")
}

pub fn fmt_execution(
	commands: &[CommandRecord],
	max_records: usize,
	max_output_lines: usize,
	max_output_chars: usize,
) -> String {
	if commands.is_empty() {
		return colorize(DIM, "No commands have been executed yet.\n");
	}
	let start = commands.len().saturating_sub(max_records);
	let recent = &commands[start..];
	let mut output = String::new();
	let _ = writeln!(
		output,
		"{} {}",
		colorize(BOLD, "COMMANDS:"),
		colorize(
			DIM,
			format!(
				"{} record(s); showing the most recent {}.",
				commands.len(),
				recent.len(),
			),
		),
	);
	for (index, command) in recent.iter().enumerate() {
		let _ = writeln!(
			output,
			"\n{} {}",
			colorize(BOLD, &format!("COMMAND {}", start + index + 1)),
			colorize(DIM, &command.command),
		);
		if let Some(code) = command.exit_code {
			let _ = writeln!(
				output,
				"{} {}",
				colorize(BOLD, "Exit code:"),
				colorize(DIM, code.to_string()),
			);
		}
		for (label, content) in [
			("stdout:", command.stdout.as_str()),
			("stderr:", command.stderr.as_str()),
		] {
			if !content.is_empty() {
				let _ = writeln!(output, "{}", colorize(BOLD, label));
				let truncated = truncate_output(content, max_output_chars);
				let preview = preview_lines(&truncated, max_output_lines);
				let _ = writeln!(output, "{}", colorize(DIM, preview));
			}
		}
	}
	output
}
pub fn fmt_history<T: std::fmt::Debug>(
	history: &[T],
	max_entries: usize,
	max_lines_per_entry: usize,
) -> String {
	let mut output = format!("{} entries\n", colorize(DIM, &history.len().to_string()),);
	let start = history.len().saturating_sub(max_entries);
	for (index, entry) in history.iter().enumerate().skip(start) {
		let rendered = format!("{entry:#?}");
		output.push_str(&format!(
			"{} {}\n",
			colorize(DIM, format!("[{index}]")),
			fmt_preview("entry", &rendered, max_lines_per_entry),
		));
	}
	if history.len() > max_entries {
		output.push_str(&format!(
			"{} more entries omitted\n",
			history.len() - max_entries,
		));
	}
	output
}
pub fn fmt_preview(label: &str, content: &str, max_lines: usize) -> String {
	format!(
		"{}:\n{}",
		colorize(BOLD, label),
		preview_lines(content, max_lines),
	)
}
pub fn fmt_workspace(workspace: &CtxWorkspace) -> String {
	let mut output = format!(
		"{} {}\n",
		colorize(BOLD, "CWD:"),
		colorize(DIM, workspace.cwd.display().to_string()),
	);
	if workspace.files.is_empty() {
		output.push_str(&format!(
			"{} {}\n",
			colorize(BOLD, "FILES:"),
			colorize(DIM, "none discovered"),
		));
	} else {
		output.push_str(&format!(
			"{} {}\n",
			colorize(BOLD, "FILES:"),
			colorize(DIM, format!("{} discovered", workspace.files.len())),
		));
		for file in &workspace.files {
			output.push_str(&format!(
				"  {} {}\n",
				colorize(DIM, "•"),
				colorize(DIM, &file.path),
			));
		}
	}
	output
}

pub fn colorize(color: &str, value: impl std::fmt::Display) -> String {
	format!("{color}{value}{RESET}")
}
pub fn truncate_output(value: &str, max_chars: usize) -> String {
	let mut chars = value.chars();
	let truncated: String = chars.by_ref().take(max_chars).collect();
	if chars.next().is_some() {
		format!("{truncated}\n[Output truncated; additional output omitted]")
	} else {
		truncated
	}
}
