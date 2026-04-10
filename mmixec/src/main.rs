mod app;
mod ui;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::{App, InputMode};

#[derive(Parser)]
#[command(name = "mmixec", about = "MMIX TUI debugger")]
struct Cli {
    /// Path to MMIX assembly file (.mms)
    file: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let source_path = std::path::Path::new(&cli.file);

    // Read source file
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| format!("Failed to read '{}': {}", cli.file, e))?;

    // Assemble
    let asm_result = mmixal::assemble(&source)
        .map_err(|e| format!("Assembly error: {}", e))?;

    // Write .mmb file (same name, different extension)
    let mmb_path = source_path.with_extension("mmb");
    mmixal::binary::save_mmb(&mmb_path, &asm_result, &source, &cli.file)
        .map_err(|e| format!("Failed to write '{}': {}", mmb_path.display(), e))?;

    // Load .mmb file back
    let (entry_addr, code, debug_info) = mmixal::binary::load_mmb(&mmb_path)
        .map_err(|e| format!("Failed to read '{}': {}", mmb_path.display(), e))?;

    let debug_info = debug_info.ok_or_else(|| {
        format!(
            "The generated MMIX binary '{}' does not contain debug information. \
This can happen if the file was written incorrectly, became corrupted, or was produced by an incompatible tool version. \
Try rebuilding the .mmb file from '{}', then rerun mmixec. If the problem persists, verify that your assembler and mmixec versions are compatible.",
            mmb_path.display(),
            cli.file
        )
    })?;

    // Initialize app
    let mut app = App::from_binary(entry_addr, &code, debug_info);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_event_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result?;
    Ok(())
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if app.running && !app.halted {
            // In running mode: execute steps rapidly, but poll for key interrupts
            if event::poll(Duration::from_millis(10))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        // Any key press stops running
                        app.running = false;
                        app.status_msg = "Stopped".into();
                        continue;
                    }
                }
            }
            // Execute a batch of steps
            for _ in 0..100 {
                if app.halted || !app.running {
                    break;
                }
                app.step();
                if app.is_at_breakpoint() {
                    app.running = false;
                    app.status_msg = format!(
                        "Breakpoint hit at line {}",
                        app.current_line().map(|l| l + 1).unwrap_or(0)
                    );
                    break;
                }
            }
            continue;
        }

        // Normal mode: wait for input
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.input_mode {
                InputMode::BreakpointInput => match key.code {
                    KeyCode::Enter => {
                        if let Ok(line_num) = app.bp_input.trim().parse::<usize>() {
                            if line_num > 0 && line_num <= app.source_lines.len() {
                                app.toggle_breakpoint(line_num - 1);
                            } else {
                                app.status_msg = format!("Invalid line number: {}", line_num);
                            }
                        } else {
                            app.status_msg = "Invalid input".into();
                        }
                        app.bp_input.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Esc => {
                        app.bp_input.clear();
                        app.input_mode = InputMode::Normal;
                        app.status_msg = "Breakpoint cancelled".into();
                    }
                    KeyCode::Backspace => {
                        app.bp_input.pop();
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        app.bp_input.push(c);
                    }
                    _ => {}
                },
                InputMode::Normal => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('n') => {
                        app.step();
                    }
                    KeyCode::Char('r') => {
                        if !app.halted {
                            app.running = true;
                            app.status_msg = "Running... (press any key to stop)".into();
                        }
                    }
                    KeyCode::Char('b') => {
                        app.input_mode = InputMode::BreakpointInput;
                        app.bp_input.clear();
                        app.status_msg = "Enter line number for breakpoint:".into();
                    }
                    KeyCode::Up => {
                        app.code_scroll = app.code_scroll.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        app.code_scroll = (app.code_scroll + 1)
                            .min(app.source_lines.len().saturating_sub(1));
                    }
                    KeyCode::Char('j') => {
                        app.reg_scroll = app.reg_scroll.saturating_add(1);
                    }
                    KeyCode::Char('k') => {
                        app.reg_scroll = app.reg_scroll.saturating_sub(1);
                    }
                    _ => {}
                },
            }
        }
    }
}
