use crate::{
	harness::{format_elapsed, pipeline::_Runtime, r#struct::AiView},
	prelude::{SdlcEvent as Event, *},
};

pub fn body(chunk: Rect) -> Vec<Rect> {
	RatatuiLayout::default()
		.direction(Direction::Horizontal)
		.constraints([
			Constraint::Percentage(54),
			Constraint::Length(1),
			Constraint::Percentage(45),
		])
		.split(chunk)
		.to_vec()
}
pub fn header(frame: &mut Frame<'_>, view: &AiView, area: Rect) {
	let elapsed = format_elapsed(view.runtime.time_started.elapsed());
	let line = Line::from(vec![
		Span::styled(
			format!("{:?}", view.runtime.stage),
			Style::default().add_modifier(Modifier::BOLD),
		),
		Span::raw(format!(
			" · attempt #{}/3   {}",
			view.runtime.attempt, elapsed
		)),
		Span::raw("    [q] quit  [p] pause  [l] logs"),
	]);

	frame.render_widget(Paragraph::new(line), area);
}
pub fn stepper(frame: &mut Frame<'_>, view: &AiView, area: Rect) {
	let stages = [
		Stage::Intent,
		Stage::Spec,
		Stage::Plan,
		Stage::Build,
		Stage::QA,
		Stage::Complete,
	];

	let current = view.runtime.stage;

	let current_style = match view.runtime.phase {
		Phase::Failed => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),

		Phase::Retrying => Style::default()
			.fg(Color::Yellow)
			.add_modifier(Modifier::BOLD),

		Phase::AwaitingHuman => Style::default()
			.fg(Color::Magenta)
			.add_modifier(Modifier::BOLD),

		_ => Style::default()
			.fg(Color::Yellow)
			.add_modifier(Modifier::BOLD),
	};

	let spinner_text = spinner(view.runtime.stage_time_started.elapsed());

	let mut spans = Vec::new();

	for (index, stage) in stages.iter().copied().enumerate() {
		let (symbol, style) = if stage == current {
			(format!("{spinner_text} "), current_style)
		} else if stage.is_before(current) {
			(
				"✓ ".to_string(),
				Style::default()
					.fg(Color::Green)
					.add_modifier(Modifier::BOLD),
			)
		} else {
			(
				"○ ".to_string(),
				Style::default()
					.fg(Color::DarkGray)
					.add_modifier(Modifier::DIM),
			)
		};

		spans.push(Span::styled(symbol, style));

		spans.push(Span::styled(format!("{stage:?}"), style));

		if index + 1 < stages.len() {
			spans.push(Span::styled("  →  ", Style::default().fg(Color::DarkGray)));
		}
	}

	frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
pub fn left_stage_panel(frame: &mut Frame<'_>, view: &AiView, area: Rect) {
	let runtime = &view.runtime;

	let phase_style = phase_style(runtime.phase);

	let score = runtime
		.score
		.map(|value| format!("{value:.2}"))
		.unwrap_or_else(|| "—".into());

	let confidence = runtime
		.confidence
		.map(|value| format!("{value:.2}"))
		.unwrap_or_else(|| "—".into());

	let passed = match runtime.passed {
		Some(true) => Span::styled(
			"yes",
			Style::default()
				.fg(Color::Green)
				.add_modifier(Modifier::BOLD),
		),

		Some(false) => Span::styled(
			"no",
			Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
		),

		None => Span::styled("—", Style::default().fg(Color::DarkGray)),
	};

	let status = match runtime.phase {
		Phase::Executing => {
			let frame = spinner(runtime.stage_time_started.elapsed());

			Line::from(vec![
				Span::styled(
					format!("{frame} "),
					Style::default()
						.fg(Color::Yellow)
						.add_modifier(Modifier::BOLD),
				),
				Span::styled(
					format!("{:?}", runtime.stage),
					Style::default()
						.fg(Color::White)
						.add_modifier(Modifier::BOLD),
				),
				Span::styled(
					format!("   attempt {}/3", runtime.attempt),
					Style::default().fg(Color::Gray),
				),
			])
		}
		Phase::Retrying => Line::from(vec![
			Span::styled(
				"↻ ",
				Style::default()
					.fg(Color::Yellow)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled(
				format!("{:?}", runtime.stage),
				Style::default()
					.fg(Color::Yellow)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled(
				format!("   retrying · attempt {}/3", runtime.attempt),
				Style::default().fg(Color::Gray),
			),
		]),
		Phase::Evaluating => Line::from(vec![
			Span::styled(
				"◆ ",
				Style::default()
					.fg(Color::Cyan)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled(
				format!("{:?}", runtime.stage),
				Style::default()
					.fg(Color::White)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled("   evaluating", Style::default().fg(Color::Cyan)),
		]),
		Phase::AwaitingHuman => Line::from(vec![
			Span::styled(
				"⚠ ",
				Style::default()
					.fg(Color::Magenta)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled(
				format!("{:?}", runtime.stage),
				Style::default()
					.fg(Color::Magenta)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled("  Input", Style::default().fg(Color::Gray)),
		]),
		Phase::Failed => Line::from(vec![
			Span::styled(
				"✗ ",
				Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
			),
			Span::styled(
				format!("{:?}", runtime.stage),
				Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
			),
			Span::styled("   failed", Style::default().fg(Color::Gray)),
		]),
		Phase::Starting => Line::from(vec![
			Span::styled("· ", Style::default().fg(Color::Yellow)),
			Span::styled(
				format!("{:?}", runtime.stage),
				Style::default()
					.fg(Color::White)
					.add_modifier(Modifier::BOLD),
			),
		]),
		Phase::Completed => Line::from(vec![
			Span::styled("✓ ", Style::default().fg(Color::Green)),
			Span::styled(
				format!("{:?}", runtime.stage),
				Style::default()
					.fg(Color::Green)
					.add_modifier(Modifier::BOLD),
			),
		]),
	};

	let lines = vec![
		status,
		Line::from(""),
		Line::from(vec![
			Span::styled("phase       ", Style::default().fg(Color::DarkGray)),
			Span::styled(format!("{:?}", runtime.phase), phase_style),
		]),
		Line::from(vec![
			Span::styled("stage time  ", Style::default().fg(Color::DarkGray)),
			Span::styled(
				format_elapsed(runtime.stage_time_started.elapsed()),
				Style::default().fg(Color::White),
			),
		]),
		Line::from(vec![
			Span::styled("JEV score   ", Style::default().fg(Color::DarkGray)),
			Span::styled(score, Style::default().fg(Color::Cyan)),
		]),
		Line::from(vec![
			Span::styled("confidence  ", Style::default().fg(Color::DarkGray)),
			Span::styled(confidence, Style::default().fg(Color::Cyan)),
		]),
		Line::from(vec![
			Span::styled("passed      ", Style::default().fg(Color::DarkGray)),
			passed,
		]),
	];
	frame.render_widget(
		Paragraph::new(lines).block(
			Block::default()
				.borders(Borders::ALL)
				.border_type(BorderType::Rounded)
				.border_style(Style::default().fg(Color::Gray))
				.style(Style::default().bg(Color::Black))
				.title(Span::styled(
					" Current Stage ",
					Style::default()
						.fg(Color::White)
						.add_modifier(Modifier::BOLD),
				)),
		),
		area,
	);
}
pub fn right_activity_panel(frame: &mut Frame<'_>, view: &AiView, area: Rect) {
	let runtime = &view.runtime;
	let spinner = spinner(runtime.stage_time_started.elapsed());
	let stage_style = Style::default()
		.fg(Color::White)
		.add_modifier(Modifier::BOLD);
	let attempt_style = Style::default().fg(Color::DarkGray);
	let phase_style = phase_style(runtime.phase);
	let phase_label = match runtime.phase {
		Phase::Starting => "Starting",
		Phase::Executing => "Executing",
		Phase::Evaluating => "Evaluating",
		Phase::Retrying => "Retrying",
		Phase::AwaitingHuman => "Awaiting input",
		Phase::Completed => "Completed",
		Phase::Failed => "Failed",
	};
	let mut lines = Vec::new();
	lines.push(Line::from(vec![
		Span::styled(
			format!("{spinner} "),
			Style::default()
				.fg(Color::Yellow)
				.add_modifier(Modifier::BOLD),
		),
		Span::styled(format!("{:?}", runtime.stage), stage_style),
		Span::styled(format!(" · #{}", runtime.attempt), attempt_style),
	]));
	lines.push(Line::from(vec![
		Span::raw("  ↳ "),
		Span::styled(phase_label, phase_style),
	]));
	if let Some(message) = &runtime.message {
		lines.push(Line::from(""));
		lines.push(Line::from(Span::styled(
			"  Why",
			Style::default()
				.fg(Color::Gray)
				.add_modifier(Modifier::BOLD),
		)));

		for line in message.lines() {
			lines.push(Line::from(vec![
				Span::raw("     "),
				Span::styled(line, Style::default().fg(Color::DarkGray)),
			]));
		}
	}
	if !runtime.activity.is_empty() {
		lines.push(Line::from(""));
		lines.push(Line::from(Span::styled(
			"  Progress",
			Style::default()
				.fg(Color::Gray)
				.add_modifier(Modifier::BOLD),
		)));

		for activity in runtime.activity.iter().rev().take(8) {
			lines.push(Line::from(vec![
				Span::raw("     • "),
				Span::styled(activity.as_str(), Style::default().fg(Color::White)),
			]));
		}
	}
	frame.render_widget(
		Paragraph::new(lines).wrap(Wrap { trim: true }).block(
			Block::default()
				.borders(Borders::ALL)
				.border_type(BorderType::Plain)
				.border_style(Style::default().fg(Color::DarkGray))
				.title(Span::styled(" Activity ", Style::default().fg(Color::Gray))),
		),
		area,
	);
}
pub fn footer(frame: &mut Frame<'_>, view: &AiView, area: Rect) {
	if view.is_input_active() {
		let input = Paragraph::new(view.input.as_str())
			.block(
				Block::default()
					.borders(Borders::ALL)
					.title("Human input — Enter to send"),
			)
			.wrap(Wrap { trim: false });
		frame.render_widget(input, area);
		let x = area.x + 1 + view.input.chars().count() as u16;
		let y = area.y + 1;
		frame.set_cursor_position(Position::new(x, y));
	} else {
		let footer = Paragraph::new("p pause  l logs  Ctrl+C quit");
		frame.render_widget(footer, area);
	}
}
pub fn event_line(event: &Event) -> Line<'static> {
	match event {
		Event::RunAttempt { stage, phase } => Line::from(vec![
			Span::styled("↳ ", Style::default().fg(Color::Magenta)),
			Span::styled(
				format!("{stage:?} human input"),
				Style::default()
					.fg(Color::Magenta)
					.add_modifier(Modifier::BOLD),
			),
			Span::raw(": "),
			// Span::styled(stage, Style::default().fg(Color::Gray)),
		]),
		Event::HumanInput { stage, input } => Line::from(vec![
			Span::styled("↳ ", Style::default().fg(Color::Magenta)),
			Span::styled(
				format!("{stage:?} human input"),
				Style::default()
					.fg(Color::Magenta)
					.add_modifier(Modifier::BOLD),
			),
			Span::raw(": "),
			Span::styled(input.clone(), Style::default().fg(Color::Gray)),
		]),
		Event::InterventionRequired {
			stage,
			attempt,
			reason,
		} => Line::from(vec![
			Span::styled(
				"⚠ ",
				Style::default()
					.fg(Color::Magenta)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled(
				format!("{stage:?} requires human intervention"),
				Style::default()
					.fg(Color::Magenta)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled(
				format!(" · attempt #{attempt}"),
				Style::default().fg(Color::Gray),
			),
			Span::raw(format!(": {reason}")),
		]),

		Event::InterventionResolved { stage, action } => Line::from(vec![
			Span::styled("✓ ", Style::default().fg(Color::Magenta)),
			Span::styled(
				format!("{stage:?} intervention"),
				Style::default()
					.fg(Color::Magenta)
					.add_modifier(Modifier::BOLD),
			),
			Span::raw(format!(" → {action}")),
		]),
		Event::RunBegin => Line::from(Span::styled(
			"SDLC started",
			Style::default().fg(Color::Gray),
		)),

		Event::StageStarted { stage, attempt } => Line::from(vec![
			Span::styled("● ", Style::default().fg(Color::White)),
			Span::styled(
				format!("{stage:?}"),
				Style::default()
					.fg(Color::White)
					.add_modifier(Modifier::BOLD),
			),
			Span::styled(
				format!(" · attempt #{attempt}"),
				Style::default().fg(Color::Gray),
			),
		]),

		Event::PhaseChanged { stage, phase } => Line::from(vec![
			Span::styled("  phase ", Style::default().fg(Color::DarkGray)),
			Span::styled(format!("→ {phase:?}"), phase_style(*phase)),
		]),

		Event::ExecutionComplete { stage } => Line::from(vec![
			Span::styled("✓ ", Style::default().fg(Color::Green)),
			Span::styled(
				format!("{stage:?} execution complete"),
				Style::default().fg(Color::Green),
			),
		]),

		Event::ExecutionFailed {
			stage,
			attempt,
			error,
		} => Line::from(vec![
			Span::styled("✗ ", Style::default().fg(Color::Red)),
			Span::styled(
				format!("{stage:?} attempt #{attempt}"),
				Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
			),
			Span::styled(format!(": {error}"), Style::default().fg(Color::Gray)),
		]),

		Event::EvaluationStarted { stage } => Line::from(vec![
			Span::styled("◆ ", Style::default().fg(Color::Cyan)),
			Span::styled(
				format!("JEV evaluating {stage:?}"),
				Style::default().fg(Color::Cyan),
			),
		]),

		Event::Evaluated {
			stage,
			score,
			confidence,
			passed,
		} => {
			let style = if *passed {
				Style::default()
					.fg(Color::Green)
					.add_modifier(Modifier::BOLD)
			} else {
				Style::default()
					.fg(Color::Yellow)
					.add_modifier(Modifier::BOLD)
			};

			Line::from(vec![
				Span::styled("◆ JEV ", Style::default().fg(Color::Cyan)),
				Span::styled(
					format!("{stage:?} · score={score:.2} confidence={confidence:.2}"),
					Style::default().fg(Color::Gray),
				),
				Span::raw(" · "),
				Span::styled(if *passed { "passed" } else { "rejected" }, style),
			])
		}

		Event::EvaluationFailed { stage, error } => Line::from(vec![
			Span::styled("✗ JEV ", Style::default().fg(Color::Red)),
			Span::styled(
				format!("{stage:?}: {error}"),
				Style::default().fg(Color::Gray),
			),
		]),

		Event::StageRunRetry { stage, number } => Line::from(vec![
			Span::styled("↻ ", Style::default().fg(Color::Yellow)),
			Span::styled(
				format!("{stage:?} · retry #{number}"),
				Style::default()
					.fg(Color::Yellow)
					.add_modifier(Modifier::BOLD),
			),
		]),

		Event::StageTransitioned { from, to } => Line::from(vec![
			Span::styled("→ ", Style::default().fg(Color::Cyan)),
			Span::styled(format!("{from:?}"), Style::default().fg(Color::Gray)),
			Span::raw(" → "),
			Span::styled(
				format!("{to:?}"),
				Style::default()
					.fg(Color::White)
					.add_modifier(Modifier::BOLD),
			),
		]),

		Event::Completed => Line::from(Span::styled(
			"✓ SDLC complete",
			Style::default()
				.fg(Color::Green)
				.add_modifier(Modifier::BOLD),
		)),

		Event::RunExited { reason } => Line::from(vec![
			Span::styled("→ exited ", Style::default().fg(Color::DarkGray)),
			Span::styled(reason.clone(), Style::default().fg(Color::Gray)),
		]),

		Event::Failed { stage, error } => {
			let message = match stage {
				Some(stage) => format!("{stage:?}: {error}"),
				None => error.clone(),
			};

			Line::from(vec![
				Span::styled("✗ ", Style::default().fg(Color::Red)),
				Span::styled(message, Style::default().fg(Color::Red)),
			])
		}
		Event::Activity {
			stage,
			attempt,
			message,
		} => Line::from(Span::styled(
			format!("↳ {message}"),
			Style::default().fg(Color::Gray),
		)),
		_ => {
			todo!();
		}
	}
}
pub fn stage_style(stage: Stage, current: Stage) -> Style {
	match stage {
		stage if stage == current => Style::default()
			.fg(Color::White)
			.add_modifier(Modifier::BOLD),

		stage if stage.is_before(current) => Style::default()
			.fg(Color::Green)
			.add_modifier(Modifier::BOLD),

		_ => Style::default()
			.fg(Color::DarkGray)
			.add_modifier(Modifier::DIM),
	}
}
pub fn phase_style(phase: Phase) -> Style {
	match phase {
		Phase::Starting => Style::default()
			.fg(Color::Yellow)
			.add_modifier(Modifier::BOLD),

		Phase::Executing => Style::default()
			.fg(Color::Yellow)
			.add_modifier(Modifier::BOLD),

		Phase::Evaluating => Style::default()
			.fg(Color::Cyan)
			.add_modifier(Modifier::BOLD),

		Phase::Retrying => Style::default()
			.fg(Color::Yellow)
			.add_modifier(Modifier::BOLD),

		Phase::AwaitingHuman => Style::default()
			.fg(Color::Magenta)
			.add_modifier(Modifier::BOLD),

		Phase::Completed => Style::default()
			.fg(Color::Green)
			.add_modifier(Modifier::BOLD),

		Phase::Failed => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
	}
}
pub fn spinner(elapsed: std::time::Duration) -> &'static str {
	const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
	let index = (elapsed.as_millis() / 100) as usize % FRAMES.len();
	FRAMES[index]
}

use crossterm::{
	cursor, execute,
	terminal::{self, ClearType},
};
use tokio::sync::mpsc::UnboundedSender;

impl AiView {
	pub fn apply(&mut self, event: Event) {
		match event {
			Event::RunBegin => {
				self.runtime.phase = Phase::Starting;
				self.runtime.time_started = Instant::now();
				self.runtime.stage_time_started = Instant::now();
				self.runtime.message = Some(String::from("Run started"));
			}
			Event::StageStarted { stage, attempt } => {
				self.runtime.stage = stage;
				self.runtime.attempt = attempt.number;
				self.runtime.phase = Phase::Starting;
				self.runtime.stage_time_started = Instant::now();
				self.runtime.score = None;
				self.runtime.confidence = None;
				self.runtime.error = None;
				self.runtime.message = Some(format!("{stage:?}"));
			}
			Event::Activity {
				stage,
				attempt,
				message,
			} => {
				self.runtime.stage = stage;
				self.runtime.attempt = attempt.number;
				self.runtime.message = Some(message);
			}
			Event::PhaseChanged { stage, phase } => {
				self.runtime.phase = match phase {
					Phase::Executing => Phase::Executing,
					Phase::Evaluating => Phase::Evaluating,
					Phase::Completed => Phase::Completed,
					// Add the remaining mappings for your actual
					// Phase variants.
					_ => self.runtime.phase,
				};
				self.runtime.message = Some(format!("{phase:?}"));
			}
			Event::ExecutionComplete { stage } => {
				self.runtime.stage = stage;
				self.runtime.phase = Phase::Evaluating;
				self.runtime.message = Some(String::from("Execution complete"));
			}
			Event::EvaluationStarted { stage } => {
				self.runtime.stage = stage;
				self.runtime.phase = Phase::Evaluating;
				self.runtime.message = Some(String::from("Evaluating"));
			}
			Event::Evaluated {
				stage,
				score,
				confidence,
				passed,
			} => {
				self.runtime.stage = stage;
				self.runtime.score = Some(score);
				self.runtime.confidence = Some(confidence);
				self.runtime.message = Some(format!(
					"Evaluation: {:.2} (confidence {:.2})",
					score, confidence
				));
				if !passed {
					self.runtime.phase = Phase::Failed;
				}
			}
			Event::StageTransitioned { from: _, to } => {
				self.runtime.stage = to;
				self.runtime.stage_time_started = Instant::now();
				self.runtime.score = None;
				self.runtime.confidence = None;
				self.runtime.message = Some(format!("Starting {to:?}"));
			}
			Event::Completed => {
				self.runtime.phase = Phase::Completed;
				self.runtime.message = Some(String::from("SDLC complete"));
			}
			Event::Failed { stage, error } => {
				if let Some(stage) = stage {
					self.runtime.stage = stage;
				}

				self.runtime.phase = Phase::Failed;
				self.runtime.message = Some(error.clone());
				self.runtime.error = Some(error);
			}

			event => {
				self.runtime.events.push(event);
			}
		}
	}
	pub fn handle_input_key(
		&mut self,
		key: crossterm::event::KeyEvent,
		input_tx: &UnboundedSender<SdlcInput>,
	) -> anyhow::Result<()> {
		use crossterm::event::KeyCode;
		match key.code {
			KeyCode::Char(c) => {
				self.input.push(c);
			}

			KeyCode::Backspace => {
				self.input.pop();
			}
			KeyCode::Enter => {
				let input = std::mem::take(&mut self.input);
				input_tx.send(SdlcInput::Human(input))?;
				self.input_active = false;
			}

			KeyCode::Esc => {
				self.input_active = false;
				self.input.clear();
			}

			_ => {}
		}
		Ok(())
	}
	pub fn begin_input(&mut self) {
		self.input_active = true;
		self.input.clear();
	}
	pub fn end_input(&mut self) {
		self.input_active = false;
		self.input.clear();
	}
	pub fn is_input_active(&self) -> bool {
		self.input_active
	}
	pub fn new(runtime: &_Runtime<harness::HarnessCtx>, pipeline: &Pipeline) -> Self {
		Self {
			input_active: false,
			input: String::new(),
			paused: false,
			show_logs: false,
			events: vec![],
			runtime: runtime.view(pipeline),
		}
	}
	pub fn render(frame: &mut Frame<'_>, view: &AiView) {
		let area = frame.area();
		frame.render_widget(Clear, area);
		let chunks = RatatuiLayout::default()
			.direction(Direction::Vertical)
			.constraints([
				Constraint::Length(2),
				Constraint::Length(3),
				Constraint::Min(8),
				Constraint::Length(3),
			])
			.split(area);
		stepper(frame, view, chunks[1]);
		let body = body(chunks[2]);
		left_stage_panel(frame, view, body[0]);
		right_activity_panel(frame, view, body[2]);
		footer(frame, view, chunks[3]);
	}
	pub fn toggle_pause(&mut self) {
		self.paused = !self.paused;
	}
	pub fn toggle_logs(&mut self) {
		self.show_logs = !self.show_logs;
	}
}

impl StatusBar {
	pub fn new() -> Self {
		Self { running: true }
	}

	pub fn draw(&self) -> io::Result<()> {
		let (_, height) = terminal::size()?;
		let mut stdout = io::stdout();

		// Save current cursor position.
		execute!(stdout, cursor::SavePosition)?;

		// Leave one blank row above the clock.
		execute!(
			stdout,
			cursor::MoveTo(0, height.saturating_sub(2)),
			terminal::Clear(ClearType::CurrentLine),
		)?;

		execute!(
			stdout,
			cursor::MoveTo(0, height.saturating_sub(1)),
			terminal::Clear(ClearType::CurrentLine),
		)?;

		let time = chrono::Local::now().format("%H:%M:%S").to_string();

		let width = terminal::size()?.0 as usize;

		execute!(
			stdout,
			cursor::MoveTo(
				width.saturating_sub(time.len()) as u16,
				height.saturating_sub(1)
			),
		)?;

		print!("{time}");

		// Put the cursor back where normal output expects it.
		execute!(stdout, cursor::RestorePosition)?;

		stdout.flush()
	}

	pub fn clear(&self) -> io::Result<()> {
		let (_, height) = terminal::size()?;
		let mut stdout = io::stdout();

		execute!(
			stdout,
			cursor::SavePosition,
			cursor::MoveTo(0, height.saturating_sub(2)),
			terminal::Clear(ClearType::FromCursorDown),
			cursor::RestorePosition,
		)?;

		stdout.flush()
	}
}

impl TerminalUi {
	pub fn new() -> Self {
		Self {
			status: StatusBar::new(),
		}
	}
	pub fn println(&self, message: impl AsRef<str>) -> io::Result<()> {
		self.status.clear()?;

		println!("{}", message.as_ref());

		self.status.draw()?;

		Ok(())
	}
}

pub struct StatusBar {
	running: bool,
}

pub struct TerminalUi {
	status: StatusBar,
}

pub struct Guard;
impl Drop for Guard {
	fn drop(&mut self) {
		let _ = crossterm::terminal::disable_raw_mode();
		let mut stdout = std::io::stdout();

		let _ = crossterm::execute!(
			stdout,
			crossterm::cursor::Show,
			crossterm::terminal::LeaveAlternateScreen
		);
	}
}
pub fn render_plain_guard(
	input: &str,
	stdout: &mut std::io::Stdout,
	session_started_at: &str,
	session_started: Instant,
) -> std::io::Result<()> {
	use crossterm::{
		QueueableCommand, cursor,
		style::{Color, Print, ResetColor, SetForegroundColor},
		terminal::{self, ClearType},
	};

	let (width, height) = terminal::size()?;
	let row = height.saturating_sub(1);

	let elapsed = session_started.elapsed().as_secs();
	let elapsed_time = format!(
		"{:02}:{:02}:{:02}",
		elapsed / 3600,
		(elapsed % 3600) / 60,
		elapsed % 60,
	);

	let ticker = format!("{session_started_at}  {elapsed_time}");
	let ticker_width = ticker.len();

	stdout.queue(cursor::MoveTo(0, row))?;
	stdout.queue(terminal::Clear(ClearType::CurrentLine))?;

	stdout.queue(SetForegroundColor(Color::Green))?;
	stdout.queue(Print("> "))?;
	stdout.queue(ResetColor)?;

	let input_width = (width as usize).saturating_sub(ticker_width + 3);

	let visible_input: String = input.chars().take(input_width).collect();
	stdout.queue(Print(visible_input))?;

	if width as usize > ticker_width + 2 {
		stdout.queue(cursor::MoveTo(
			width.saturating_sub(ticker_width as u16),
			row,
		))?;
		stdout.queue(SetForegroundColor(Color::Cyan))?;
		stdout.queue(Print(ticker))?;
		stdout.queue(ResetColor)?;
	}

	stdout.queue(cursor::Show)?;
	stdout.flush()
}
