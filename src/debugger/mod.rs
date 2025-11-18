// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use std::collections::HashSet;
use std::io;
use std::time::Duration;

use crate::assembler::asm_types::InstructionIR;
use crate::cpu::CpuState;
use crate::types::{EmuError, EmuResult, Word};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
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

#[derive(Debug, Clone, Copy)]
pub enum LastCommand {
    Step,
    Continue,
    BreakpointSet(usize),
    BreakpointRemoved(usize),
    None,
}

pub struct MemoryDiffEntry {
    pub line_number: usize,
    pub diffs: Vec<(usize, u8, u8)>,
}

pub struct DebuggerState {
    pub breakpoints: HashSet<usize>,
    pub last_command: LastCommand,
    pub last_executed_insn: Option<InstructionIR>,
    pub memory_diff_history: Vec<MemoryDiffEntry>,
}

impl DebuggerState {
    pub fn new() -> Self {
        Self {
            breakpoints: HashSet::new(),
            last_command: LastCommand::None,
            last_executed_insn: None,
            memory_diff_history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Source,
    History,
    MemoryDiff,
}

#[derive(Clone)]
struct HistoryEntry {
    command: String,
    error: Option<String>,
}

struct DebuggerUIState {
    source: ListState,
    history: ListState,
    memdiff: ListState,
    focus: FocusArea,
}

impl DebuggerUIState {
    fn new() -> Self {
        let mut s = ListState::default();
        s.select(Some(0));
        Self {
            source: s,
            history: ListState::default(),
            memdiff: ListState::default(),
            focus: FocusArea::Source,
        }
    }

    fn toggle_focus(&mut self, history_len: usize, memdiff_len: usize, reverse: bool) {
        self.focus = if reverse {
            match self.focus {
                FocusArea::MemoryDiff => FocusArea::History,
                FocusArea::History => FocusArea::Source,
                FocusArea::Source => FocusArea::MemoryDiff,
            }
        } else {
            match self.focus {
                FocusArea::Source => {
                    if history_len > 0 && self.history.selected().is_none() {
                        self.history.select(Some(0));
                    }
                    FocusArea::History
                }
                FocusArea::History => {
                    if memdiff_len > 0 && self.memdiff.selected().is_none() {
                        self.memdiff.select(Some(0));
                    }
                    FocusArea::MemoryDiff
                }
                FocusArea::MemoryDiff => FocusArea::Source,
            }
        };
    }

    fn scroll_up(&mut self, (_s_len, _h_len, _m_len): (usize, usize, usize)) {
        match self.focus {
            FocusArea::Source => {
                if let Some(i) = self.source.selected() {
                    if i > 0 {
                        self.source.select(Some(i - 1));
                    }
                }
            }
            FocusArea::History => {
                if let Some(i) = self.history.selected() {
                    if i > 0 {
                        self.history.select(Some(i - 1));
                    }
                }
            }
            FocusArea::MemoryDiff => {
                if let Some(i) = self.memdiff.selected() {
                    if i > 0 {
                        self.memdiff.select(Some(i - 1));
                    }
                }
            }
        }
    }

    fn scroll_down(&mut self, (s_len, h_len, m_len): (usize, usize, usize)) {
        match self.focus {
            FocusArea::Source => {
                if let Some(i) = self.source.selected() {
                    if i + 1 < s_len {
                        self.source.select(Some(i + 1));
                    }
                }
            }
            FocusArea::History => {
                if let Some(i) = self.history.selected() {
                    if i + 1 < h_len {
                        self.history.select(Some(i + 1));
                    }
                }
            }
            FocusArea::MemoryDiff => {
                if let Some(i) = self.memdiff.selected() {
                    if i + 1 < m_len {
                        self.memdiff.select(Some(i + 1));
                    }
                }
            }
        }
    }
}

fn render_source<'a>(cpu: &'a CpuState, dbg: &'a DebuggerState) -> Vec<ListItem<'a>> {
    let ip = *cpu.ip.borrow();
    let src_context = cpu.program.ip_map.get(ip);

    if let Some(entry) = src_context {
        let file = cpu
            .program
            .files
            .iter()
            .find(|f| f.filename == entry.filename);
        if let Some(filesource) = file {
            let total_lines = filesource.lines.len();
            // Always highlight only the line for the IP-mapped instruction
            let highlight_line = entry.line.saturating_sub(1); // entry.line is 1-based

            // Clamp the view around the highlight line for a nice window
            let win = 5_usize;
            let start = highlight_line.saturating_sub(win);
            let end = (highlight_line + win + 1).min(total_lines);

            (start..end)
                .map(|abs_index| {
                    let display_line = abs_index + 1;
                    let bp = if dbg.breakpoints.contains(&display_line) {
                        "B "
                    } else {
                        "  "
                    };
                    let ptr = if abs_index == highlight_line {
                        "> "
                    } else {
                        "  "
                    };
                    let line_text = &filesource.lines[abs_index];
                    ListItem::new(format!("{bp}{ptr}{display_line:4} {line_text}"))
                })
                .collect()
        } else {
            vec![ListItem::new("(file not found for IP)")]
        }
    } else {
        vec![ListItem::new("(no source mapping for IP)")]
    }
}

fn render_registers<'a>(cpu: &'a CpuState) -> Vec<Span<'a>> {
    let regs = cpu.registers.borrow();
    let mut list: Vec<(String, Word)> = (0..32).map(|i| (format!(" X{}", i), regs[i])).collect();
    list.push((" SP".to_string(), *cpu.sp.borrow() as u64));
    list.push((" PC".to_string(), *cpu.ip.borrow() as u64));
    let mut spans = Vec::new();
    for row in 0..17 {
        let mut s = String::new();
        for col in 0..2 {
            let idx = row + col * 17;
            if idx < list.len() {
                s.push_str(&format!("{:<4}: 0x{:016X}  ", list[idx].0, list[idx].1));
            }
        }
        spans.push(Span::raw(s));
    }
    spans
}

// Optimized diff
fn compute_memory_diff_fast(a: &[u8], b: &[u8]) -> Vec<(usize, u8, u8)> {
    const CHUNK: usize = 64;
    let mut res = Vec::with_capacity(2048);
    let mut i = 0;
    while i + CHUNK <= a.len() {
        if &a[i..i + CHUNK] != &b[i..i + CHUNK] {
            for j in 0..CHUNK {
                if a[i + j] != b[i + j] {
                    res.push((i + j, a[i + j], b[i + j]));
                    if res.len() > 5000 {
                        res.push((usize::MAX, 0, 0));
                        return res;
                    }
                }
            }
        }
        i += CHUNK;
    }
    for n in i..a.len() {
        if a[n] != b[n] {
            res.push((n, a[n], b[n]));
            if res.len() > 5000 {
                res.push((usize::MAX, 0, 0));
                break;
            }
        }
    }
    res
}

fn render_diff<'a>(history: &[MemoryDiffEntry], selected: Option<usize>) -> Vec<ListItem<'a>> {
    if history.is_empty() {
        return vec![ListItem::new("(no memory diffs yet)")];
    }

    let mut items = Vec::new();
    for (i, entry) in history.iter().enumerate() {
        // Header row for the entry
        let mut lines: Vec<Line> = vec![Line::from(Span::styled(
            format!("[Line {}]", entry.line_number),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))];

        // Memory address changes
        for (addr, old, new) in &entry.diffs.iter().take(10).collect::<Vec<_>>() {
            lines.push(Line::from(Span::raw(format!(
                "  0x{:08X}: {:02X} → {:02X}",
                addr, old, new
            ))));
        }

        if entry.diffs.len() > 10 {
            lines.push(Line::from(Span::raw(format!(
                "  ... ({} more changes)",
                entry.diffs.len() - 10
            ))));
        }

        let mut item = ListItem::new(lines);
        if selected == Some(i) {
            item = item.style(
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
        }
        items.push(item);
    }
    items
}

fn handle_command(
    cpu: &mut CpuState,
    dbg: &mut DebuggerState,
    _ui: &mut DebuggerUIState,
    cmd: &str,
) -> EmuResult<(bool, LastCommand, Option<String>)> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    match parts.as_slice() {
        // Step one instruction
        ["s"] | ["step"] => {
            cpu.step_instruction()?;
            dbg.last_executed_insn = Some(cpu.current_instruction()?);
            Ok((
                true,
                LastCommand::Step,
                Some("Stepped one instruction.".into()),
            ))
        }

        // Continue until breakpoint or halt
        ["c"] | ["continue"] => {
            while !cpu.halted() {
                let ip = *cpu.ip.borrow() as usize;
                if dbg.breakpoints.contains(&ip) {
                    break;
                }
                cpu.step_instruction()?;
                dbg.last_executed_insn = Some(cpu.current_instruction()?);
            }
            if cpu.halted() {
                Ok((
                    false,
                    LastCommand::Continue,
                    Some("Program halted (exit).".into()),
                ))
            } else {
                Ok((
                    true,
                    LastCommand::Continue,
                    Some("Continued execution.".into()),
                ))
            }
        }

        // Set breakpoint
        ["b", line_str] | ["break", line_str] => {
            let line = line_str.parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!("Invalid line number: {}", line_str))
            })?;
            dbg.breakpoints.insert(line);
            Ok((
                true,
                LastCommand::BreakpointSet(line),
                Some(format!("Breakpoint set at line {}", line)),
            ))
        }

        // Delete breakpoint
        ["d", line_str] | ["delete", line_str] => {
            let line = line_str.parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!("Invalid line number: {}", line_str))
            })?;
            dbg.breakpoints.remove(&line);
            Ok((
                true,
                LastCommand::BreakpointRemoved(line),
                Some(format!("Breakpoint removed from line {}", line)),
            ))
        }

        // Examine memory (x <size> <addr>)
        ["x", size_str, addr_str] => {
            let size = size_str.parse::<usize>().map_err(|_| {
                EmuError::InternalError(format!("Invalid memory size: {}", size_str))
            })?;
            let addr = Word::from_str_radix(addr_str.trim_start_matches("0x"), 16)
                .map_err(|_| EmuError::InternalError(format!("Invalid address: {}", addr_str)))?;
            match cpu.memory.borrow().read_bytes(addr, size) {
                Ok(bytes) => {
                    let hex = bytes
                        .iter()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(" ");
                    Ok((
                        true,
                        dbg.last_command,
                        Some(format!("0x{:016X}: {}", addr, hex)),
                    ))
                }
                Err(EmuError::MemoryAccessViolation(_)) => Ok((
                    true,
                    dbg.last_command,
                    Some(format!(
                        "Error: Invalid memory address or size: 0x{:X}",
                        addr
                    )),
                )),
                Err(e) => Err(e),
            }
        }

        // Reset / Run command
        ["r"] | ["reset"] => {
            // Keep the currently loaded program and rebuild CPU state
            let program_clone = cpu.program.clone();
            let plugin_manager = cpu.plugin_manager.clone();
            let vfs = cpu.vfs.clone();

            // Create a brand-new CpuState using the same program
            let new_cpu = CpuState::new(program_clone, plugin_manager, vfs);

            // Reset breakpoints and diff history to start clean
            dbg.breakpoints.clear();
            dbg.last_executed_insn = None;
            dbg.last_command = LastCommand::None;
            dbg.memory_diff_history.clear();

            // Replace the old CPU with the new instance contents
            *cpu = new_cpu;

            Ok((
                true,
                LastCommand::None,
                Some("CPU state reset — program reloaded.".into()),
            ))
        }

        // Quit commands
        ["q"] | ["quit"] | ["exit"] => {
            Ok((false, dbg.last_command, Some("Exiting debugger.".into())))
        }

        // Help message (restored full version)
        ["h"] | ["help"] | ["?"] => {
            let msg = "Commands:
  [ENTER]/s: Repeat last command.
  r/reset: Reset CPU and program.
  s/step: Execute one instruction.
  c/continue: Run continuously.
  b/break <Line>: Set breakpoint.
  d/delete <Line>: Delete breakpoint.
  x <size> <addr>: Examine memory.
  q/quit/exit: Exit debugger.
  h/help/?: Show this help.
Use Tab/Shift+Tab to switch panels, ↑/↓ to scroll, Enter to run commands."
                .to_string();
            Ok((true, dbg.last_command, Some(msg)))
        }

        // Unknown command fallback
        _ => {
            let joined = parts.join(" ");
            let err = format!("Unknown command: {}. Type 'h' or 'help'.", joined);
            Ok((true, dbg.last_command, Some(err)))
        }
    }
}

pub fn run_debugger(cpu: &mut CpuState) -> EmuResult<()> {
    let mut dbg = DebuggerState::new();
    let mut ui = DebuggerUIState::new();
    let mut hist = Vec::new();
    let mut memdiff = Vec::new();
    let mut prev_mem = cpu.memory.borrow().ram.clone();
    let mut input = String::new();
    let mut last_cmd: Option<String> = None;
    let mut done = false;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    while !done {
        term.draw(|f| {
            let size = f.area();
            let top = Layout::default()
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
            let bottom = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)])
                .split(Rect {
                    x: 0,
                    y: size.height / 2,
                    width: size.width,
                    height: size.height / 2,
                });

            let border = |fa: FocusArea| {
                if ui.focus == fa {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }
            };

            // Source list with highlight
            let src_list = List::new(render_source(cpu, &dbg))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(FocusArea::Source))
                        .title("Source Code"),
                )
                .highlight_symbol(">> ")
                .highlight_spacing(HighlightSpacing::Always)
                .highlight_style(
                    Style::default()
                        .bg(Color::Yellow)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_stateful_widget(src_list, top[0], &mut ui.source);

            // Registers
            let regs: Vec<Line> = render_registers(cpu).into_iter().map(Line::from).collect();
            f.render_widget(
                Paragraph::new(regs)
                    .block(Block::default().borders(Borders::ALL).title("Registers")),
                top[1],
            );

            // Memory diff with highlight
            let diff_list = List::new(render_diff(&dbg.memory_diff_history, ui.memdiff.selected()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(FocusArea::MemoryDiff))
                        .title("Memory Diff Log"),
                )
                .highlight_symbol(">> ")
                .highlight_spacing(HighlightSpacing::Always)
                .highlight_style(
                    Style::default()
                        .bg(Color::Yellow)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(diff_list, top[2], &mut ui.memdiff);

            // History with highlight
            let items: Vec<ListItem> = hist
                .iter()
                .map(|h: &HistoryEntry| {
                    if let Some(e) = &h.error {
                        ListItem::new(format!("{} [ERROR: {}]", h.command, e))
                    } else {
                        ListItem::new(h.command.clone())
                    }
                })
                .collect();
            let hist_list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(FocusArea::History))
                        .title("History"),
                )
                .highlight_symbol(">> ")
                .highlight_spacing(HighlightSpacing::Always)
                .highlight_style(
                    Style::default()
                        .bg(Color::Yellow)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_stateful_widget(hist_list, bottom[0], &mut ui.history);

            let inp = Paragraph::new(input.as_str()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Command Input"),
            );
            f.render_widget(inp, bottom[1]);
        })?;

        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    match k.code {
                        KeyCode::Char('q') => done = true,
                        KeyCode::Char(c) => input.push(c),
                        KeyCode::Backspace => {
                            input.pop();
                        }
                        KeyCode::Enter => {
                            let t = input.trim();
                            let cmd = if t.is_empty() {
                                last_cmd.clone().unwrap_or_default()
                            } else {
                                t.to_string()
                            };
                            if !cmd.is_empty() {
                                let snap = { cpu.memory.borrow().ram.clone() };
                                let new_diffs = compute_memory_diff_fast(&prev_mem, &snap);
                                if !new_diffs.is_empty() {
                                    let current_ip = *cpu.ip.borrow();
                                    let line_number = cpu
                                        .program
                                        .ip_map
                                        .iter()
                                        .find(|entry| entry.ip == current_ip)
                                        .map(|entry| entry.line)
                                        .unwrap_or(0); // fallback if unmapped
                                    dbg.memory_diff_history.push(MemoryDiffEntry {
                                        line_number,
                                        diffs: new_diffs.clone(),
                                    });
                                    const MAX_DIFF_HISTORY: usize = 200;
                                    if dbg.memory_diff_history.len() > MAX_DIFF_HISTORY {
                                        // Efficiently remove oldest entries without reallocating
                                        let excess =
                                            dbg.memory_diff_history.len() - MAX_DIFF_HISTORY;
                                        dbg.memory_diff_history.drain(0..excess);
                                    }

                                    memdiff.extend(new_diffs);
                                }
                                prev_mem.copy_from_slice(&snap);
                                match handle_command(cpu, &mut dbg, &mut ui, &cmd) {
                                    Ok((cont, lc, msg)) => {
                                        hist.push(HistoryEntry {
                                            command: cmd.clone(),
                                            error: None,
                                        });
                                        if let Some(m) = msg {
                                            hist.push(HistoryEntry {
                                                command: m,
                                                error: None,
                                            });
                                        }
                                        last_cmd = Some(cmd);
                                        dbg.last_command = lc;
                                        if !cont {
                                            done = true;
                                        }
                                    }
                                    Err(e) => hist.push(HistoryEntry {
                                        command: cmd.clone(),
                                        error: Some(e.to_string()),
                                    }),
                                }
                            }
                            input.clear();
                        }
                        KeyCode::Tab => ui.toggle_focus(hist.len(), memdiff.len(), false),
                        KeyCode::BackTab => ui.toggle_focus(hist.len(), memdiff.len(), true),
                        KeyCode::Up | KeyCode::Down => {
                            // Get current file's source line count (or fallback to 0)
                            let current_ip = *cpu.ip.borrow();
                            let source_len = cpu
                                .program
                                .ip_map
                                .iter()
                                .find(|entry| entry.ip == current_ip)
                                .and_then(|entry| {
                                    cpu.program
                                        .files
                                        .iter()
                                        .find(|f| f.filename == entry.filename)
                                        .map(|fs| fs.lines.len())
                                })
                                .unwrap_or(0);
                            if k.code == KeyCode::Up {
                                ui.scroll_up((source_len, hist.len(), memdiff.len()));
                            } else {
                                ui.scroll_down((source_len, hist.len(), memdiff.len()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    Ok(())
}
