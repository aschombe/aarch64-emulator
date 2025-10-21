use std::collections::{HashMap, HashSet};
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
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph},
};

use crate::assembler::asm_types::InstructionIR;
use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, Word};

#[derive(Debug, Clone, Copy)]
pub enum LastCommand {
    Step,
    Continue,
    BreakpointSet(usize),
    BreakpointRemoved(usize),
    ExamineMemory(usize, usize),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Source,
    History,
}

#[derive(Clone)]
struct HistoryEntry {
    command: String,
    error: Option<String>,
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

    fn scroll_up_source(&mut self, _source_len: usize) {
        if let Some(i) = self.source_list_state.selected() {
            if i > 0 {
                self.source_list_state.select(Some(i - 1));
            }
        }
    }

    fn scroll_down_source(&mut self, source_len: usize) {
        if let Some(i) = self.source_list_state.selected() {
            if i + 1 < source_len {
                self.source_list_state.select(Some(i + 1));
            }
        }
    }

    fn scroll_up_history(&mut self, _history_len: usize) {
        if let Some(i) = self.history_list_state.selected() {
            if i > 0 {
                self.history_list_state.select(Some(i - 1));
            }
        }
    }

    fn scroll_down_history(&mut self, history_len: usize) {
        if let Some(i) = self.history_list_state.selected() {
            if i + 1 < history_len {
                self.history_list_state.select(Some(i + 1));
            }
        }
    }

    fn toggle_focus(&mut self, history_len: usize) {
        match self.focus {
            FocusArea::Source => {
                if self.history_list_state.selected().is_none() && history_len > 0 {
                    self.history_list_state.select(Some(0));
                }
                self.focus = FocusArea::History;
            }
            FocusArea::History => {
                self.focus = FocusArea::Source;
            }
        }
    }
}

fn render_history_scrollable<'a>(
    history: &'a [HistoryEntry],
    selected: Option<usize>,
) -> Vec<ListItem<'a>> {
    history
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            // Convert multiline command/output into Lines of Spans
            let mut lines: Vec<Line> = entry
                .command
                .lines()
                .map(|line| Line::from(Span::raw(line)))
                .collect();

            // Append error message in last line if exists
            if let Some(err) = &entry.error {
                if let Some(last_line) = lines.last_mut() {
                    let mut spans = last_line.spans.clone();
                    spans.push(Span::raw(format!("  [ERROR: {}]", err)));
                    *last_line = Line {
                        spans,
                        ..*last_line
                    };
                } else {
                    lines.push(Line::from(Span::raw(format!("[ERROR: {}]", err))));
                }
            }

            let mut list_item = ListItem::new(lines);

            if selected == Some(idx) {
                list_item = list_item.style(
                    Style::default()
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );
            } else {
                list_item = list_item.style(Style::default());
            }
            list_item
        })
        .collect()
}

fn find_next_valid_line(
    source_map: &Vec<usize>, // instruction index → source line number
    source_line_to_ui_map: &HashMap<usize, usize>,
    current_ip: usize,
) -> Option<usize> {
    let max_ip = source_map.len();
    for ip in current_ip..max_ip {
        if let Some(&ui_idx) = source_line_to_ui_map.get(&source_map[ip]) {
            return Some(ui_idx);
        }
    }
    None
}

fn source_line_to_ui_index_map(source_lines: &[String]) -> HashMap<usize, usize> {
    let mut map = HashMap::new();
    let mut ui_index = 0;
    for (idx, line) in source_lines.iter().enumerate() {
        let line_num = idx + 1; // 1-based line number
        if !line.trim().is_empty() {
            map.insert(line_num, ui_index);
            ui_index += 1;
        }
    }
    map
}

fn render_source_scrollable<'a>(cpu: &'a CpuState, dbg: &'a DebuggerState) -> Vec<ListItem<'a>> {
    let source_line_to_ui_idx = source_line_to_ui_index_map(&cpu.program.source_lines);
    let current_ip = cpu.ip;
    let current_source_line = cpu.program.source_map.get(current_ip).copied().unwrap_or(0);
    let highlight_index =
        find_next_valid_line(&cpu.program.source_map, &source_line_to_ui_idx, current_ip);

    cpu.program
        .source_lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let line_num = idx + 1;
            let bp_marker = if dbg.breakpoints.contains(&line_num) {
                "B "
            } else {
                "  "
            };

            let ip_marker = if highlight_index == source_line_to_ui_idx.get(&line_num).copied() {
                "> "
            } else {
                "  "
            };

            let content = format!("{}{}{:4} {}", bp_marker, ip_marker, line_num, line);
            ListItem::new(content)
        })
        .collect()
}

fn render_registers<'a>(cpu: &'a CpuState) -> Vec<Span<'a>> {
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

// New helper to synchronize UI highlight immediately after stepping
fn update_highlight_ui(cpu: &CpuState, ui_state: &mut DebuggerUIState) {
    let source_line_to_ui_idx = source_line_to_ui_index_map(&cpu.program.source_lines);
    let current_ip = cpu.ip;
    let source_map = &cpu.program.source_map;

    // Try to get UI index from current instruction's source line
    let current_source_line = source_map.get(current_ip).copied().unwrap_or(0);
    let highlight_idx = source_line_to_ui_idx
        .get(&current_source_line)
        .copied()
        // Fallback to next valid line if current line blank or missing in map
        .or_else(|| find_next_valid_line(source_map, &source_line_to_ui_idx, current_ip));

    ui_state.source_list_state.select(highlight_idx);
}

fn execute_step(
    cpu: &mut CpuState,
    dbg: &mut DebuggerState,
    ui_state: &mut DebuggerUIState,
) -> EmuResult<()> {
    cpu.step_instruction()?;
    dbg.last_executed_insn = Some(cpu.current_instruction()?);
    update_highlight_ui(cpu, ui_state);
    Ok(())
}

fn execute_continue(
    cpu: &mut CpuState,
    dbg: &mut DebuggerState,
    ui_state: &mut DebuggerUIState,
) -> EmuResult<()> {
    while !cpu.halted() {
        if dbg.breakpoints.contains(&(cpu.ip as usize)) {
            break;
        }
        cpu.step_instruction()?;
        dbg.last_executed_insn = Some(cpu.current_instruction()?);
    }
    update_highlight_ui(cpu, ui_state);
    Ok(())
}

fn handle_debug_command(
    cpu: &mut CpuState,
    dbg: &mut DebuggerState,
    ui_state: &mut DebuggerUIState,
    command: &str,
) -> EmuResult<(bool, LastCommand, Option<String>)> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    match parts.as_slice() {
        ["s"] | ["step"] => {
            execute_step(cpu, dbg, ui_state)?;
            Ok((
                true,
                LastCommand::Step,
                Some("Stepped one instruction".to_string()),
            ))
        }
        ["c"] | ["continue"] => {
            execute_continue(cpu, dbg, ui_state)?;
            if cpu.halted() {
                // Exit the debugger main loop when halted
                Ok((
                    false,
                    LastCommand::Continue,
                    Some("Program halted (exit).".to_string()),
                ))
            } else {
                Ok((
                    true,
                    LastCommand::Continue,
                    Some("Continued execution.".to_string()),
                ))
            }
        }
        ["b", line_str] | ["break", line_str] => {
            let line = line_str.parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!("Invalid line number format: {}", line_str))
            })?;
            dbg.breakpoints.insert(line);
            Ok((
                true,
                LastCommand::BreakpointSet(line),
                Some(format!("Breakpoint set at line {}", line)),
            ))
        }
        ["d", line_str] | ["delete", line_str] => {
            let line = line_str.parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!("Invalid line number format: {}", line_str))
            })?;
            dbg.breakpoints.remove(&line);
            Ok((
                true,
                LastCommand::BreakpointRemoved(line),
                Some(format!("Breakpoint removed from line {}", line)),
            ))
        }
        ["x", size_str, addr_str] => {
            let size = size_str.parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!("Invalid memory size format: {}", size_str))
            })?;

            let addr = Word::from_str_radix(addr_str.trim_start_matches("0x"), 16)
                .map_err(|_| EmuError::InternalError(format!("Invalid address: {}", addr_str)))?;

            match cpu.memory.read_bytes(addr, size) {
                Ok(bytes) => Ok((
                    true,
                    dbg.last_command,
                    Some(format!("0x{:016X}: {:?}", addr, bytes)),
                )),
                Err(EmuError::MemoryAccessViolation(_)) => Ok((
                    false,
                    dbg.last_command,
                    Some(format!(
                        "Error: Invalid memory address or size: 0x{:X}",
                        addr
                    )),
                )),
                Err(e) => Err(e),
            }
        }
        ["q"] | ["quit"] | ["exit"] => Ok((false, dbg.last_command, None)),

        ["h"] | ["help"] | ["?"] => {
            let help_text = "Commands:
  [ENTER]/s: Repeat last command.
  s/step: Execute one instruction.
  c/continue: Run continuously.
  b/break <Line>: Set breakpoint.
  d/delete <Line>: Delete breakpoint.
  x <size> <addr>: Examine memory.
  q/quit/exit: Exit debugger."
                .to_string();
            Ok((true, dbg.last_command, Some(help_text)))
        }
        _ => {
            let joined = parts.join(" ");
            let err_msg = format!("Unknown command: {}. Type 'h' or 'help'.", joined);
            Ok((true, dbg.last_command, Some(err_msg)))
        }
    }
}

pub fn run_debugger(cpu: &mut CpuState) -> EmuResult<()> {
    let mut debugger_state = DebuggerState::new();
    let mut ui_state = DebuggerUIState::new();
    let mut history: Vec<HistoryEntry> = Vec::new();
    let mut input = String::new();
    let mut last_command_ran: Option<String> = None;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut break_loop = false;

    loop {
        terminal.draw(|f| {
            let size = f.area();

            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(35),
                    Constraint::Percentage(40),
                ])
                .split(Rect {
                    x: 0,
                    y: 0,
                    width: size.width,
                    height: size.height / 2,
                });

            let bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)])
                .split(Rect {
                    x: 0,
                    y: size.height / 2,
                    width: size.width,
                    height: size.height / 2,
                });

            let source_items = render_source_scrollable(cpu, &debugger_state);
            let source_list = List::new(source_items)
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

            let regs_spans = render_registers(cpu);
            let regs_lines: Vec<Line> = regs_spans.into_iter().map(Line::from).collect();
            let regs_paragraph = Paragraph::new(regs_lines)
                .block(Block::default().borders(Borders::ALL).title("Registers"));
            f.render_widget(regs_paragraph, top_chunks[1]);

            // let data_output = if let Some(data_bytes) = cpu.memory.data_section() {
            //     let region_base = cpu
            //         .memory
            //         .regions
            //         .iter()
            //         .find(|r| r.name == "data")
            //         .map(|r| r.base)
            //         .unwrap_or(0);
            //
            //     let mut lines = Vec::new();
            //     for (i, chunk) in data_bytes.chunks(16).enumerate() {
            //         let addr = region_base + i as u64 * 16;
            //         let hex_bytes = chunk
            //             .iter()
            //             .map(|b| format!("{:02X}", b))
            //             .collect::<Vec<_>>()
            //             .join(" ");
            //         lines.push(format!("0x{:08X}: {}", addr, hex_bytes));
            //     }
            //     lines.join("\n")
            // } else {
            //     "No .data section".to_string()
            // };
            //
            // let data_paragraph = Paragraph::new(data_output)
            //     .block(Block::default().borders(Borders::ALL).title("Data Dump"))
            //     .style(Style::default().fg(Color::White));
            //
            // f.render_widget(data_paragraph, top_chunks[2]);

            let history_selected = ui_state.history_list_state.selected();
            let history_items = render_history_scrollable(&history, history_selected);
            let history_list = List::new(history_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("History")
                        .border_style(if ui_state.focus == FocusArea::History {
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
                .highlight_symbol(">> ")
                .highlight_spacing(HighlightSpacing::Always);
            f.render_stateful_widget(
                history_list,
                bottom_chunks[0],
                &mut ui_state.history_list_state,
            );

            let input_block = Block::default()
                .title("Command Input")
                .borders(Borders::ALL);
            let input_paragraph = Paragraph::new(input.as_str()).block(input_block);
            f.render_widget(input_paragraph, bottom_chunks[1]);
        })?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break_loop = true,
                    KeyCode::Char(c) => input.push(c),
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        let trimmed = input.trim();
                        let cmd = if trimmed.is_empty() {
                            last_command_ran.clone().unwrap_or_default()
                        } else {
                            trimmed.to_string()
                        };

                        if !cmd.is_empty() {
                            match handle_debug_command(
                                cpu,
                                &mut debugger_state,
                                &mut ui_state,
                                &cmd,
                            ) {
                                Ok((continue_running, last_cmd, output_opt)) => {
                                    history.push(HistoryEntry {
                                        command: cmd.clone(),
                                        error: None,
                                    });
                                    if let Some(out) = output_opt {
                                        history.push(HistoryEntry {
                                            command: out,
                                            error: None,
                                        });
                                    }
                                    ui_state.history_list_state.select(Some(history.len() - 1));

                                    last_command_ran = Some(cmd);
                                    debugger_state.last_command = last_cmd;

                                    if !continue_running {
                                        break_loop = true;
                                    }
                                }
                                Err(e) => {
                                    history.push(HistoryEntry {
                                        command: cmd.clone(),
                                        error: Some(e.to_string()),
                                    });
                                    ui_state.history_list_state.select(Some(history.len() - 1));
                                    last_command_ran = Some(cmd);
                                }
                            }
                        }
                        input.clear();
                    }
                    KeyCode::Up => match ui_state.focus {
                        FocusArea::Source => {
                            ui_state.scroll_up_source(cpu.program.source_lines.len())
                        }
                        FocusArea::History => ui_state.scroll_up_history(history.len()),
                    },
                    KeyCode::Down => match ui_state.focus {
                        FocusArea::Source => {
                            ui_state.scroll_down_source(cpu.program.source_lines.len())
                        }
                        FocusArea::History => ui_state.scroll_down_history(history.len()),
                    },
                    KeyCode::Tab => {
                        ui_state.toggle_focus(history.len());
                    }
                    _ => {}
                }
            }
        }

        if break_loop {
            break;
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
