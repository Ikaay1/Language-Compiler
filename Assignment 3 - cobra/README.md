# Cobra Compiler (Week 3)

This project implements a compiler for the **Cobra language**, extending Boa with booleans, conditionals, loops, mutation, and runtime type checking.

---

## Features

- Tagged value representation
- Booleans: `true`, `false`
- Input support
- Unary operations:
  - `add1`, `sub1`, `negate`, `isnum`, `isbool`
- Binary operations:
  - `+`, `-`, `*`, `<`, `>`, `<=`, `>=`, `=`
- `if` expressions
- `block` expressions
- `loop` / `break`
- `set!` mutation
- Runtime errors for:
  - Invalid arguments
  - Arithmetic overflow

---

## Tagged Value Representation

Cobra uses **tagged values** to distinguish numbers from booleans at runtime.

### Encoding

- **Numbers** are encoded by shifting left by 1 bit
  - Example: `5` → `10`

- **Booleans** are encoded as odd values:
  - `false = 1`
  - `true = 3`

### Interpretation

- Values with **LSB = 0** → Numbers
- Values with **LSB = 1** → Booleans

---

## Runtime Errors

The compiler reports:

- `invalid argument` → Type errors
- `overflow` → Arithmetic overflow

---

## Example Programs

### Boolean

```lisp
(if true 5 10)
```

### Comparison

```lisp
(< 3 5)
```

### Loop / Break

```lisp
(let ((x 0))
  (loop
    (if (= x 10)
        (break x)
        (set! x (+ x 1)))))
```

### Mutation

```lisp
(let ((x 5))
  (block
    (set! x 9)
    x))
```

---

## Build

```bash
cargo build
```

---

## Generate Assembly

```bash
cargo run -- test/if_true.snek test/if_true.s
```

---

## Run Tests

```bash
make test
```
