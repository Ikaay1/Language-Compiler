# Snek Compiler (Adder)

A small-scale compiler that translates S-expression-based mathematics (`.snek` files) into x86_64 assembly for macOS. This project demonstrates the fundamentals of parsing, code generation, and linking against a Rust runtime.

## Overview

The compiler pipeline follows a standard flow:

1. **Parsing**: Converts S-expressions into an internal Abstract Syntax Tree (AST) using the `sexp` crate.
2. **Code Generation**: Recursively generates x86_64 assembly instructions, primarily utilizing the `rax` register for computations.
3. **Assembly & Linking**: Uses `nasm` to assemble the output and `rustc` to link it against a minimal Rust runtime (`start.rs`) that handles the final output display.

## Supported Operations

The compiler currently supports the following numeric operations:

| Operation         | Description               | Assembly Implementation |
| :---------------- | :------------------------ | :---------------------- |
| `(add1 <expr>)`   | Increments the value by 1 | `add rax, 1`            |
| `(sub1 <expr>)`   | Decrements the value by 1 | `sub rax, 1`            |
| `(negate <expr>)` | Negates the value         | `neg rax`               |
| `<number>`        | A raw 32-bit integer      | `mov rax, <n>`          |

## Project Structure

- `src/main.rs`: Core compiler logic (parser and code generator).
- `runtime/start.rs`: Entry point that invokes the compiled assembly and prints the result.
- `Makefile`: Automates the compilation and linking process.
- `test/`: Directory for source `.snek` files and generated outputs.

## Getting Started

### Prerequisites

- **Rust** (Cargo)
- **NASM** (x86_64 assembler)
- **macOS** environment (Makefile targets `macho64` and `x86_64-apple-darwin`).

### Building and Running

To compile a specific `.snek` file into a runnable program:

```bash
# Example: Compile and run test/add1.snek
make test/add1.run
./test/add1.run
```

### Cleanup

To remove generated assembly files, libraries, and binaries:

```bash
make clean
```

## Examples

### Complex Expression

**Input (`test/complex.snek`):**
`(sub1 (negate (add1 1)))`

**Compiled Assembly:**

```nasm
section .text
global our_code_starts_here
our_code_starts_here:
  mov rax, 1
  add rax, 1
  neg rax
  sub rax, 1
  ret
```

**Result:** `-3`
