# MMIX

> This project constructed mostly by Copilot

A Rust implementation of the [MMIX](https://mmix.cs.hm.edu/) virtual machine — the educational 64-bit RISC computer designed by Donald Knuth for *The Art of Computer Programming*.

This project provides a complete toolchain: an assembler, a simulator, and an interactive TUI debugger with source-level stepping and breakpoints.

## Features

- **MMIX Simulator** — Faithful emulation of the MMIX instruction set with cycle and memory-access counting
- **MMIXAL Assembler** — Assembles MMIX assembly (`.mms`) into a binary format (`.mmb`) with embedded debug information
- **TUI Debugger** — Step through programs instruction-by-instruction, set breakpoints, inspect registers, and view output — all from the terminal

## Project Structure

| Crate | Description |
|---|---|
| `mmix_core` | Core VM: instruction decoding, execution, memory model, registers |
| `mmixal` | Assembler: parses `.mms` source and produces `.mmb` binaries with source mapping |
| `mmixec` | Interactive TUI debugger built with [Ratatui](https://ratatui.rs/) |
| `mmix_macros` | Proc macros for generating opcode tables and register definitions |

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition)

### Build

```sh
cargo build --release
```

### Run the Debugger

```sh
cargo run --bin mmixec -- examples/hello.mms
```

This launches the TUI debugger with the `hello.mms` example program.

### Debugger Controls

| Key | Action |
|---|---|
| `s` | Step one instruction |
| `r` | Run until breakpoint or halt |
| `b` | Enter breakpoint input mode (type a line number, then Enter) |
| `q` | Quit |

## Example

`examples/hello.mms` demonstrates basic MMIX programming:

```asm
        GETA    $255,String
        TRAP    0,1,1
        SET     $2,3
        SET     $3,4
        ADD     $4,$2,$3
        SET     $10,5
Loop    SUB     $10,$10,1
        BNZ     $10,Loop
        TRAP    0,0,0
String  BYTE    "Hello, World!",#a,0
```

## Binary Format (`.mmb`)

The assembler produces `.mmb` files containing:

- Machine code
- Entry point address
- Debug metadata: source line ↔ byte offset mappings and embedded source text

This enables the debugger to display original source alongside execution state.

## Architecture

```
 .mms source
     │
     ▼
  mmixal ──► .mmb binary
     │
     ▼
  mmix_core (VM)
     │
     ▼
  mmixec (TUI debugger)
```

## License

See repository for license details.
