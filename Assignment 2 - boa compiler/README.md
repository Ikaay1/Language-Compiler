# Boa Compiler (Assignment 2)

This project implements a **compiler for the Boa language**, extending the Week 1 Adder compiler. The compiler reads a Boa program written in S-expression syntax (`.snek` files), converts it into an abstract syntax tree (AST), and generates **x86-64 assembly**.

The resulting assembly can then be assembled and linked to produce an executable that evaluates the Boa program.

---

## Overview

Boa adds several key features beyond the basic arithmetic expressions from Week 1:

- **Variables** (identifiers)
- **Let bindings** for introducing variables and scopes
- **Binary operations** (`+`, `-`, `*`)
- **Stack-based storage** for variables
- **Environment management** for variable lookup

The compiler evaluates expressions and produces assembly instructions that leave the final result in the **`RAX` register**.

---

## Language Grammar

### Concrete Syntax

```
<expr> :=
  | <number>
  | <identifier>
  | (let ((<identifier> <expr>)+) <expr>)
  | (add1 <expr>)
  | (sub1 <expr>)
  | (+ <expr> <expr>)
  | (- <expr> <expr>)
  | (* <expr> <expr>)
```

### Identifiers

```
[a-zA-Z][a-zA-Z0-9_-]*
```

Reserved keywords:

```
let
add1
sub1
```

---

## Abstract Syntax Tree (AST)

The Boa program is parsed into the following AST representation:

```rust
enum Op1 {
    Add1,
    Sub1,
}

enum Op2 {
    Plus,
    Minus,
    Times,
}

enum Expr {
    Number(i32),
    Id(String),
    Let(Vec<(String, Expr)>, Box<Expr>),
    UnOp(Op1, Box<Expr>),
    BinOp(Op2, Box<Expr>, Box<Expr>),
}
```

---

## Compiler Architecture

The compiler consists of three main stages.

### 1. Parsing

Input `.snek` files are parsed as S-expressions and converted into the AST.

Examples:

```
42
```

```
(add1 5)
```

```
(+ 1 2)
```

```
(let ((x 5) (y 6)) (+ x y))
```

---

### 2. Environment Management

Variables are stored on the **stack** and tracked using an immutable environment.

```
env: HashMap<String, i32>
```

The environment maps identifiers to **stack offsets**.

Example:

```
x → -16
y → -24
```

This allows the compiler to load variable values from memory.

---

### 3. Code Generation

The compiler produces x86-64 assembly instructions.

Each expression compiles to instructions that leave the result in **`RAX`**.

Example translations:

```
Number(5)
→ mov rax, 5
```

```
(add1 5)
→ mov rax, 5
→ add rax, 1
```

Binary operations use temporary stack storage:

```
(+ 1 2)

mov rax, 1
mov [rsp - 16], rax
mov rax, 2
add rax, [rsp - 16]
```

---

## Stack Layout

The stack grows downward from `rsp`.

```
Higher addresses
-----------------
[rsp - 8]   reserved
[rsp -16]   first variable
[rsp -24]   second variable
[rsp -32]   third variable
-----------------
Lower addresses
```

Each stack slot uses **8 bytes**.

---

## Example Program

Input (`example.snek`):

```
(let ((x 5)
      (y (+ x 1)))
  (* x y))
```

Execution:

```
x = 5
y = 6
result = 30
```

---

## Building the Compiler

Compile the project:

```
cargo build
```

---

## Running Unit Tests

The project includes Rust unit tests for parsing and compilation behavior.

Run them with:

```
cargo test
```

---

## Compiling a Boa Program

To compile a `.snek` file into assembly:

```
cargo run -- test/simple.snek test/simple.s
```

This produces an assembly file:

```
test/simple.s
```

---

## Building the Executable

Assemble and link the program using `make`:

```
make test/simple.run
```

---

## Running the Program

Execute the compiled program:

```
./test/simple.run
```

Expected output:

```
42
```

---

## Running All Provided Tests

Run the full test suite using:

```
make test
```

This compiles, assembles, and runs all test programs in the `test/` directory.

---

## Error Handling

The compiler detects several types of invalid programs.

| Error Type         | Behavior                                       |
| ------------------ | ---------------------------------------------- |
| Invalid syntax     | `panic!("Invalid")`                            |
| Duplicate bindings | `panic!("Duplicate binding")`                  |
| Unbound variable   | `panic!("Unbound variable identifier {name}")` |

---

## Example Test Programs

Example `.snek` programs:

```
42
```

```
(add1 (add1 3))
```

```
(+ (* 2 3) 3)
```

```
(let ((x 5)) (+ x x))
```

```
(let ((x 5) (y 6)) (+ x y))
```
