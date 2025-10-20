// Left Top Pane: Source code with breakpoints and current line highlighted
// Left Bottom Pane: Legend of commands and symbols
// Right Top Left Pane: Register values in columns
// Right Top Right Pane: Data section memory dump
// Right Bottom Pane: Command input and history

use std::collections::HashSet;
use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::assembler::asm_types::InstructionIR;
use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, Word};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Source,
    History,
}

#[derive(Debug, Clone, Copy)]
pub enum LastCommand {
    Step,
    Continue,
    None,
}

pub struct DebuggerState {
    pub breakpoints: HashSet<usize>,
    pub last_command: LastCommand,
    pub running_continuously: bool,
    pub last_executed_insn: Option<InstructionIR>,
}

impl DebuggerState {
    pub fn new() -> Self {
        Self {
            breakpoints: HashSet::new(),
            last_command: LastCommand::None,
            running_continuously: false,
            last_executed_insn: None,
        }
    }
}

struct DebuggerUIState {
    source_list_state: ListState,
    history_list_state: ListState,
    focus: FocusArea,
}

impl DebuggerUIState {
    fn new() -> Self {
        let mut source_list_state = ListState::default();
        source_list_state.select(Some(0));
        let mut history_list_state = ListState::default();
        history_list_state.select(None);

        Self {
            source_list_state,
            history_list_state,
            focus: FocusArea::Source,
        }
    }

    fn scroll_up(&mut self) {
        match self.focus {
            FocusArea::Source => {
                let i = self.source_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.source_list_state.select(Some(i - 1));
                }
            }
            FocusArea::History => {
                let i = self.history_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.history_list_state.select(Some(i - 1));
                }
            }
        }
    }

    fn scroll_down(&mut self, max: usize) {
        match self.focus {
            FocusArea::Source => {
                let i = self.source_list_state.selected().unwrap_or(0);
                if i + 1 < max {
                    self.source_list_state.select(Some(i + 1));
                }
            }
            FocusArea::History => {
                let i = self.history_list_state.selected().unwrap_or(0);
                if i + 1 < max {
                    self.history_list_state.select(Some(i + 1));
                }
            }
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            FocusArea::Source => FocusArea::History,
            FocusArea::History => FocusArea::Source,
        };
    }
}

fn handle_debug_command(
    cpu: &mut CpuState,
    dbg: &mut DebuggerState,
    command: &str,
) -> EmuResult<(bool, LastCommand)> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    if let Some(cmd) = parts.first() {
        if cmd.starts_with('b') || cmd == &"break" || cmd.starts_with('d') || cmd == &"delete" {
            let target = parts.get(1).map(|s| *s).ok_or_else(|| {
                EmuError::InternalError("Breakpoint command requires a line number.".to_string())
            })?;
            let target_line = target
                .parse::<usize>()
                .map_err(|_| EmuError::InternalError(format!("Invalid line number: {}", target)))?;
            if cmd.starts_with('b') || cmd == &"break" {
                dbg.breakpoints.insert(target_line);
            } else {
                dbg.breakpoints.remove(&target_line);
            }
            return Ok((false, dbg.last_command));
        }
    }

    Ok((false, dbg.last_command))
}

fn render_source_scrollable<'a>(cpu: &'a CpuState, dbg: &'a DebuggerState) -> Vec<ListItem<'a>> {
    let source_lines = &cpu.program.source_lines;
    let source_map = &cpu.program.source_map;
    source_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_num = i + 1;
            let bp_marker = if dbg.breakpoints.contains(&line_num) {
                "B "
            } else {
                "  "
            };
            let current_line = source_map.get(cpu.ip).cloned().unwrap_or(0);
            let ip_marker = if line_num == current_line { "> " } else { "  " };
            let content = format!("{}{}{:4} {}", bp_marker, ip_marker, line_num, line);
            ListItem::new(content)
        })
        .collect()
}

fn render_history_scrollable(history: &[String]) -> Vec<ListItem> {
    history.iter().map(|s| ListItem::new(s.clone())).collect()
}

fn render_registers_aligned<'a>(cpu: &'a CpuState) -> Vec<Span<'a>> {
    // Registers X0-X31 + SP in two columns aligned with space and padding
    let mut regs: Vec<(String, Word)> = (0..32)
        .map(|i| (format!(" X{}", i), cpu.registers[i]))
        .collect();
    regs.push((" SP".to_string(), cpu.sp));
    regs.push((" PC".to_string(), cpu.ip as u64));

    let rows = 17;
    let columns = 2;
    let mut spans = Vec::new();

    for row in 0..rows {
        let mut line = String::new();
        for col in 0..columns {
            let idx = row + col * rows;
            if idx < regs.len() {
                let (ref name, val) = regs[idx];
                line.push_str(&format!("{:<4}: 0x{:016X}  ", name, val));
            }
        }
        spans.push(Span::raw(line));
    }
    spans
}

pub fn run_debugger(cpu: &mut CpuState) -> EmuResult<()> {
    let mut debugger_state = DebuggerState::new();
    let mut ui_state = DebuggerUIState::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut input = String::new();
    let mut history: Vec<String> = Vec::new();

    loop {
        terminal.draw(|f| {
            let size = f.size();

            // Layout: Top half horizontal split: 25% code, 40% blank, 35% registers
            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                // .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
                .constraints(
                    [
                        Constraint::Percentage(25),
                        Constraint::Percentage(40),
                        Constraint::Percentage(35),
                    ]
                    .as_ref(),
                )
                .split(Rect {
                    x: 0,
                    y: 0,
                    width: size.width,
                    height: size.height / 2,
                });

            // Draw scrollable source code list with highlight
            let source_items = render_source_scrollable(&cpu, &debugger_state);
            let mut source_list = List::new(source_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Source Code")
                        .border_style(if ui_state.focus == FocusArea::Source {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">>");
            f.render_stateful_widget(source_list, top_chunks[0], &mut ui_state.source_list_state);

            // Registers pane
            let reg_block = Block::default().borders(Borders::ALL).title("Registers");
            let regs_spans = render_registers_aligned(&cpu);
            // let reg_paragraph = Paragraph::new(regs_spans).block(reg_block);
            let regs_lines: Vec<Line> = regs_spans.into_iter().map(Line::from).collect();
            let regs_paragraph = Paragraph::new(regs_lines).block(reg_block);

            f.render_widget(regs_paragraph, top_chunks[2]);

            // Bottom half vertical split: history and input
            let bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                .split(Rect {
                    x: 0,
                    y: size.height / 2,
                    width: size.width,
                    height: size.height / 2,
                });

            // History list with focus border highlight
            let history_items = render_history_scrollable(&history);
            let mut history_list = List::new(history_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("History")
                    .border_style(if ui_state.focus == FocusArea::History {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            );
            f.render_stateful_widget(
                history_list,
                bottom_chunks[0],
                &mut ui_state.history_list_state,
            );

            // Command input box
            let input_block = Block::default()
                .title("Command Input")
                .borders(Borders::ALL);
            let input_paragraph = Paragraph::new(input.as_str()).block(input_block);
            f.render_widget(input_paragraph, bottom_chunks[1]);
        })?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        history.push(input.clone());
                        if history.len() > 1000 {
                            history.remove(0);
                        }
                        match handle_debug_command(cpu, &mut debugger_state, input.trim()) {
                            Ok((do_exec, last_cmd)) => {
                                debugger_state.last_command = last_cmd;
                                debugger_state.running_continuously =
                                    matches!(last_cmd, LastCommand::Continue);
                                if do_exec {
                                    // Add stepping/continuation logic here
                                }
                            }
                            Err(_) => {
                                // handle errors here
                            }
                        }
                        input.clear();
                    }
                    KeyCode::Up => {
                        ui_state.scroll_up();
                    }
                    KeyCode::Down => {
                        let max = match ui_state.focus {
                            FocusArea::Source => cpu.program.source_lines.len(),
                            FocusArea::History => history.len(),
                        };
                        ui_state.scroll_down(max);
                    }
                    KeyCode::Tab => {
                        ui_state.toggle_focus();
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
