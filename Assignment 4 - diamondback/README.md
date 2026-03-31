# Diamondback Compiler (Week 4)

This project implements a compiler for the **Diamondback** language, extending the Week 3 Cobra compiler with **function definitions**, **function calls**, and a stack-frame-based **calling convention**.

The compiler reads a Diamondback program written in S-expression syntax (`.snek` files), parses it into an abstract syntax tree, and generates **x86-64 assembly**.

---

## Features

- Numbers, booleans, and `input`
- Unary operations: `add1`, `sub1`, `negate`, `isnum`, `isbool`, `print`
- Binary operations: `+`, `-`, `*`, `<`, `>`, `<=`, `>=`, `=`
- `if` expressions
- `block` expressions
- `loop` / `break`
- `set!` mutation
- Top-level function definitions
- Function calls with fixed arity
- Recursive and mutually recursive functions
- Runtime errors for invalid arguments and overflow

---

## Language Grammar

A Diamondback program consists of zero or more function definitions followed by one main expression.

### Program

```
<prog> := <defn>* <expr>
```

### Function Definition

```
<defn> := (fun (<name> <name>*) <expr>)
```

### Expressions

```
<expr> :=
  | <number> | true | false | input
  | <identifier>
  | (let ((<identifier> <expr>)+) <expr>)
  | (<op1> <expr>)
  | (<op2> <expr> <expr>)
  | (if <expr> <expr> <expr>)
  | (block <expr>+)
  | (loop <expr>)
  | (break <expr>)
  | (set! <identifier> <expr>)
  | (<name> <expr>*)
```

---

## Tagged Value Representation

Diamondback uses tagged values to distinguish numbers from booleans at runtime.

- **Numbers** are encoded by shifting left by 1 bit — e.g. `5 → 10`
- **Booleans** are odd values: `false = 1`, `true = 3`

Values with LSB `0` are numbers; values with LSB `1` are booleans.

---

## Calling Convention

Diamondback uses a stack-based calling convention with `rbp`.

### Caller Responsibilities

For a call like `(add3 1 2 3)`, the caller evaluates arguments, pushes them right-to-left, calls the function, then cleans up the stack:

```asm
mov rax, 6
push rax
mov rax, 4
push rax
mov rax, 2
push rax
call fun_add3
add rsp, 24
```

### Callee Responsibilities

```asm
fun_name:
  push rbp
  mov rbp, rsp
  ...
  pop rbp
  ret
```

### Stack Frame Layout

```
Higher addresses
---------------
rbp+24  | argument 2
---------------
rbp+16  | argument 1
---------------
rbp+8   | return address
---------------
rbp     | saved rbp
---------------
rbp-8   | local variable 1
---------------
rbp-16  | local variable 2
---------------
Lower addresses
```

- First parameter → `rbp + 16`
- Second parameter → `rbp + 24`
- Locals → negative offsets

---

## Compiler Design

1. **Parsing** — Parses `.snek` into a `Program` containing `Definition`s and `Expression`s
2. **Environment Management** — Tracks variables: parameters at positive offsets, locals at negative offsets
3. **Function Compilation** — Each function compiled into a labeled block with prologue/epilogue
4. **Call Compilation** — Arguments compiled, pushed in reverse order, stack cleaned after return
5. **Main Expression** — Compiled into `our_code_starts_here`

---

## Runtime Support

- `snek_error` — prints error message and exits
- `snek_print` — prints a value and returns it

---

## Examples

### Simple Function

```scheme
(
  (fun (double x) (+ x x))
  (double 5)
)
```

**Result:** `10`

### Multiple Arguments

```scheme
(
  (fun (add3 x y z)
    (+ (+ x y) z))
  (add3 1 2 3)
)
```

**Result:** `6`

### Recursion

```scheme
(
  (fun (fact n)
    (if (= n 1)
        1
        (* n (fact (sub1 n)))))
  (fact 5)
)
```

**Result:** `120`

### Mutual Recursion

```scheme
(
  (fun (even n)
    (if (= n 0)
        true
        (odd (sub1 n))))
  (fun (odd n)
    (if (= n 0)
        false
        (even (sub1 n))))
  (even 10)
)
```

**Result:** `true`

### Local Variables

```scheme
(
  (fun (compute x)
    (let ((y (* x 2))
          (z (+ x 1)))
      (+ y z)))
  (compute 5)
)
```

**Result:** `16`

### Printing

```scheme
(
  (fun (show x) (print x))
  (show 7)
)
```

**Output:**

```
7
7
```

---

## Error Handling

- Wrong number of arguments
- Undefined function
- Invalid arguments / type mismatches
- Arithmetic overflow

---

## Building & Running

```bash
# Build
cargo build

# Compile a program
cargo run -- test/fun1.snek test/fun1.s

# Run tests
make test
```

---

## Project Structure

```
src/main.rs        — compiler
runtime/start.rs   — runtime support
Makefile           — build & test targets
test/              — example programs
```

---

## Summary

This project extends Cobra into a full Diamondback compiler supporting user-defined functions, stack-frame management, argument passing, recursion, and runtime error/print support — the first version with full structured programs and reusable functions.
