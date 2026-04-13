use std::collections::{HashMap, HashSet};
use mmix_core::{Machine, SpecialRegister};
use mmixal::DebugInfo;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    BreakpointInput,
}

pub struct App {
    pub machine: Machine,
    pub source_lines: Vec<String>,
    pub addr_to_line: HashMap<u64, usize>,
    pub breakpoints: HashSet<usize>,
    pub output: String,
    pub running: bool,
    pub step_mode: bool,
    pub halted: bool,
    pub status_msg: String,
    pub input_mode: InputMode,
    pub bp_input: String,
    pub code_scroll: usize,
    pub reg_scroll: usize,
    pub error_msg: Option<String>,
}

impl App {
    pub fn from_binary(entry_addr: u64, code: &[u8], debug_info: DebugInfo) -> Self {
        let mut machine = Machine::new();
        machine.load_raw(entry_addr, code);
        machine.set_entry(entry_addr);

        Self {
            machine,
            source_lines: debug_info.source_lines,
            addr_to_line: debug_info.offset_to_line,
            breakpoints: HashSet::new(),
            output: String::new(),
            running: false,
            step_mode: false,
            halted: false,
            status_msg: String::from("Ready. n=step r=run b=breakpoint q=quit"),
            input_mode: InputMode::Normal,
            bp_input: String::new(),
            code_scroll: 0,
            reg_scroll: 0,
            error_msg: None,
        }
    }

    pub fn step(&mut self) {
        if self.halted {
            self.status_msg = "Program halted".into();
            return;
        }

        match self.machine.step() {
            Ok(()) => {
                // Collect output from machine buffer
                if !self.machine.output_buffer.is_empty() {
                    let text = String::from_utf8_lossy(&self.machine.output_buffer).to_string();
                    self.output.push_str(&text);
                    self.machine.output_buffer.clear();
                }

                if self.machine.halted {
                    self.halted = true;
                    self.running = false;
                    self.step_mode = false;
                    self.status_msg = "Program halted".into();
                } else {
                    let pc = self.machine.pc;
                    if let Some(line) = self.addr_to_line.get(&pc) {
                        self.status_msg = format!("PC={:#x} line {}", pc, line + 1);
                    } else {
                        self.status_msg = format!("PC={:#x}", pc);
                    }
                }
                self.error_msg = None;
            }
            Err(e) => {
                self.halted = true;
                self.running = false;
                self.step_mode = false;
                self.error_msg = Some(e.clone());
                self.status_msg = format!("Error: {}", e);
            }
        }
    }

    pub fn current_line(&self) -> Option<usize> {
        let pc = self.machine.pc;
        self.addr_to_line.get(&pc).copied()
    }

    /// Run continuously, stopping at breakpoints or halt.
    /// Uses `Machine::run_until()` to delegate execution to the machine layer.
    pub fn run_continuous(&mut self) {
        if self.halted {
            self.status_msg = "Program halted".into();
            return;
        }

        let breakpoints = &self.breakpoints;
        let addr_to_line = &self.addr_to_line;
        let mut hit_breakpoint = false;

        let result = if breakpoints.is_empty() {
            self.machine.run_until(|_| false)
        } else {
            self.machine.run_until(|m| {
                if let Some(&line) = addr_to_line.get(&m.pc) {
                    if breakpoints.contains(&line) {
                        hit_breakpoint = true;
                        return true;
                    }
                }
                false
            })
        };

        // Collect output
        if !self.machine.output_buffer.is_empty() {
            let text = String::from_utf8_lossy(&self.machine.output_buffer).to_string();
            self.output.push_str(&text);
            self.machine.output_buffer.clear();
        }

        match result {
            Ok(()) => {
                if self.machine.halted {
                    self.halted = true;
                    self.running = false;
                    self.step_mode = false;
                    self.status_msg = "Program halted".into();
                } else if hit_breakpoint {
                    self.step_mode = true;
                    self.status_msg = format!(
                        "Breakpoint hit at line {} (step mode: n=step r=run q=quit)",
                        self.current_line().map(|l| l + 1).unwrap_or(0)
                    );
                }
                self.error_msg = None;
            }
            Err(e) => {
                self.halted = true;
                self.running = false;
                self.step_mode = false;
                self.error_msg = Some(e.clone());
                self.status_msg = format!("Error: {}", e);
            }
        }
    }

    pub fn toggle_breakpoint(&mut self, line: usize) {
        if self.breakpoints.contains(&line) {
            self.breakpoints.remove(&line);
            self.status_msg = format!("Breakpoint removed at line {}", line + 1);
        } else {
            self.breakpoints.insert(line);
            self.status_msg = format!("Breakpoint set at line {}", line + 1);
        }
    }

    /// Get non-zero general registers as (index, value) pairs
    pub fn nonzero_general_regs(&self) -> Vec<(u8, u64)> {
        (0..=255u8)
            .filter_map(|i| {
                let v = self.machine.general.get(i);
                if v != 0 { Some((i, v)) } else { None }
            })
            .collect()
    }

    /// Get all special registers as (name, value) pairs
    pub fn all_special_regs(&self) -> Vec<(&'static str, u64)> {
        SpecialRegister::ALL
            .iter()
            .map(|sr| (sr.name(), self.machine.special.get(*sr)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app(src: &str) -> App {
        let asm_result = mmixal::assemble(src).unwrap();
        let debug_info = DebugInfo {
            line_to_offset: asm_result.line_to_offset.clone(),
            offset_to_line: asm_result.offset_to_line.clone(),
            source_file: String::new(),
            source_lines: src.lines().map(|s| s.to_string()).collect(),
        };
        App::from_binary(asm_result.entry_addr, &asm_result.bytes, debug_info)
    }

    #[test]
    fn new_app_initial_state() {
        let app = make_app("        TRAP 0,0,0");
        assert!(!app.halted);
        assert!(!app.running);
        assert!(!app.step_mode);
        assert_eq!(app.output, "");
        assert_eq!(app.breakpoints.len(), 0);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.entry_addr(), 0);
    }

    #[test]
    fn step_halts_on_trap_0() {
        let mut app = make_app("        TRAP 0,0,0");
        app.step();
        assert!(app.halted);
    }

    #[test]
    fn step_after_halt() {
        let mut app = make_app("        TRAP 0,0,0");
        app.step();
        assert!(app.halted);
        app.step();
        assert_eq!(app.status_msg, "Program halted");
    }

    #[test]
    fn step_advances_pc() {
        let src = "        SETL $1,42\n        TRAP 0,0,0";
        let mut app = make_app(src);
        assert_eq!(app.current_line(), Some(0));
        app.step();
        assert_eq!(app.current_line(), Some(1));
    }

    #[test]
    fn step_executes_setl() {
        let mut app = make_app("        SETL $1,42\n        TRAP 0,0,0");
        app.step();
        assert_eq!(app.machine.general.get(1), 42);
    }

    #[test]
    fn step_add_computation() {
        let src = "\
        SETL $1,3\n\
        SETL $2,4\n\
        ADD  $3,$1,$2\n\
        TRAP 0,0,0";
        let mut app = make_app(src);
        app.step(); // SETL $1,3
        app.step(); // SETL $2,4
        app.step(); // ADD $3,$1,$2
        assert_eq!(app.machine.general.get(3), 7);
    }

    #[test]
    fn toggle_breakpoint() {
        let mut app = make_app("        SETL $1,1\n        TRAP 0,0,0");
        app.toggle_breakpoint(0);
        assert!(app.breakpoints.contains(&0));
        app.toggle_breakpoint(0);
        assert!(!app.breakpoints.contains(&0));
    }

    #[test]
    fn source_lines() {
        let src = "        SETL $1,1\n        TRAP 0,0,0";
        let app = make_app(src);
        assert_eq!(app.source_lines.len(), 2);
        assert_eq!(app.source_lines[0].trim(), "SETL $1,1");
    }

    #[test]
    fn nonzero_general_regs() {
        let src = "        SETL $5,100\n        TRAP 0,0,0";
        let mut app = make_app(src);
        app.step();
        let regs = app.nonzero_general_regs();
        assert!(regs.iter().any(|&(i, v)| i == 5 && v == 100));
    }

    #[test]
    fn loop_program_runs_to_completion() {
        let src = "\
        SETL    $10,5\n\
Loop    SUB     $10,$10,1\n\
        BNZ     $10,Loop\n\
        TRAP    0,0,0";
        let mut app = make_app(src);
        // Run until halted (max 100 steps as safety)
        for _ in 0..100 {
            if app.halted { break; }
            app.step();
        }
        assert!(app.halted);
        assert_eq!(app.machine.general.get(10), 0);
    }

    #[test]
    fn trap_fputs_collects_output() {
        let src = "\
        GETA    $255,String\n\
        TRAP    0,1,1\n\
        TRAP    0,0,0\n\
String  BYTE    \"Hi\",0";
        let mut app = make_app(src);
        app.step(); // GETA
        app.step(); // TRAP 0,1,1 (Fputs)
        assert!(app.output.contains("Hi"));
    }

    impl App {
        fn entry_addr(&self) -> u64 {
            self.machine.pc - if self.halted { 4 } else { 0 }
            // just checks initial state is 0
        }
    }

    #[test]
    fn step_halts_resets_running_and_step_mode() {
        let mut app = make_app("        TRAP 0,0,0");
        app.running = true;
        app.step_mode = true;
        app.step();
        assert!(app.halted);
        assert!(!app.running);
        assert!(!app.step_mode);
    }

    #[test]
    fn run_continuous_to_halt() {
        let src = "        SETL $1,42\n        TRAP 0,0,0";
        let mut app = make_app(src);
        app.running = true;
        app.run_continuous();
        assert!(app.halted);
        assert!(!app.running);
        assert!(!app.step_mode);
        assert_eq!(app.machine.general.get(1), 42);
    }

    #[test]
    fn run_continuous_stops_at_breakpoint() {
        let src = "        SETL $1,1\n        SETL $2,2\n        TRAP 0,0,0";
        let mut app = make_app(src);
        app.running = true;
        app.toggle_breakpoint(1); // breakpoint at line 1 (SETL $2,2)
        app.run_continuous();
        assert!(!app.halted);
        assert!(app.running); // still running
        assert!(app.step_mode); // switched to step mode
        assert_eq!(app.current_line(), Some(1)); // stopped at breakpoint line
    }
}
