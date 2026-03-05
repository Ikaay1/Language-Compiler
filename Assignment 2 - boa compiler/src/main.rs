// Assignment 2: Boa Compiler - Starter Code
// TODO: Complete this compiler implementation
//
// Your task is to implement a compiler for the Boa language
// that compiles expressions with let bindings to x86-64 assembly.
//
// Boa extends Adder with:
//   - Variables (identifiers)
//   - Let expressions with multiple bindings
//   - Binary operations: +, -, *

use im::HashMap;
use sexp::Atom::*;
use sexp::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;

// ============= Abstract Syntax Tree =============

/// Unary operators
#[derive(Debug)]
enum Op1 {
    Add1,
    Sub1,
}

/// Binary operators
#[derive(Debug)]
enum Op2 {
    Plus,
    Minus,
    Times,
}

/// The Boa expression AST
///
/// Grammar:
///   <expr> := <number>
///           | <identifier>
///           | (let (<binding>+) <expr>)
///           | (add1 <expr>) | (sub1 <expr>)
///           | (+ <expr> <expr>) | (- <expr> <expr>) | (* <expr> <expr>)
///   <binding> := (<identifier> <expr>)
#[derive(Debug)]
enum Expr {
    Number(i32),
    Id(String),
    Let(Vec<(String, Expr)>, Box<Expr>),
    UnOp(Op1, Box<Expr>),
    BinOp(Op2, Box<Expr>, Box<Expr>),
}

// ============= Assembly Representation =============

/// Values that can appear in assembly instructions
#[derive(Debug)]
enum Val {
    Reg(Reg),
    Imm(i32),
    RegOffset(Reg, i32), // e.g., [rsp - 8]
}

/// Registers we use
#[derive(Debug)]
enum Reg {
    RAX,
    RSP,
}

/// Assembly instructions we generate
#[derive(Debug)]
enum Instr {
    IMov(Val, Val),
    IAdd(Val, Val),
    ISub(Val, Val),
    IMul(Val, Val),
}

// ============= Parsing =============

fn is_reserved(name: &str) -> bool {
    matches!(name, "let" | "add1" | "sub1")
}

/// Parse an S-expression into our Expr AST
///
/// Error handling:
///   - Invalid syntax: panic!("Invalid")
///   - Number out of i32 range: panic!("Invalid")
fn parse_expr(s: &Sexp) -> Expr {
    match s {
        Sexp::Atom(I(n)) => {
            let val = i32::try_from(*n).unwrap_or_else(|_| panic!("Invalid"));
            Expr::Number(val)
        }

        Sexp::Atom(S(name)) => {
            if is_reserved(name) {
                panic!("Invalid");
            }
            Expr::Id(name.to_string())
        }

        Sexp::List(vec) => match &vec[..] {
            // unary ops
            [Sexp::Atom(S(op)), e] if op == "add1" => Expr::UnOp(Op1::Add1, Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "sub1" => Expr::UnOp(Op1::Sub1, Box::new(parse_expr(e))),

            // binary ops
            [Sexp::Atom(S(op)), e1, e2] if op == "+" => Expr::BinOp(
                Op2::Plus,
                Box::new(parse_expr(e1)),
                Box::new(parse_expr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "-" => Expr::BinOp(
                Op2::Minus,
                Box::new(parse_expr(e1)),
                Box::new(parse_expr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "*" => Expr::BinOp(
                Op2::Times,
                Box::new(parse_expr(e1)),
                Box::new(parse_expr(e2)),
            ),

            // let
            [Sexp::Atom(S(op)), Sexp::List(binds), body] if op == "let" => {
                // (let ((x e) (y e) ...) body)
                let mut parsed: Vec<(String, Expr)> = Vec::new();
                for b in binds {
                    parsed.push(parse_bind(b));
                }
                if parsed.is_empty() {
                    panic!("Invalid");
                }
                Expr::Let(parsed, Box::new(parse_expr(body)))
            }

            _ => panic!("Invalid"),
        },

        _ => panic!("Invalid"),
    }
}

/// Parse a single binding from a let expression
///
/// Error handling:
///   - Invalid binding syntax: panic!("Invalid")
fn parse_bind(s: &Sexp) -> (String, Expr) {
    match s {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(name)), e] => {
                if is_reserved(name) {
                    panic!("Invalid");
                }
                (name.to_string(), parse_expr(e))
            }
            _ => panic!("Invalid"),
        },
        _ => panic!("Invalid"),
    }
}

// ============= Compilation =============

fn stack_offset(si: i32) -> i32 {
    -8 * si
}

/// Compile an expression to a list of assembly instructions
fn compile_to_instrs(e: &Expr, si: i32, env: &HashMap<String, i32>) -> Vec<Instr> {
    match e {
        Expr::Number(n) => vec![Instr::IMov(Val::Reg(Reg::RAX), Val::Imm(*n))],

        Expr::Id(name) => {
            let offset = *env
                .get(name)
                .unwrap_or_else(|| panic!("Unbound variable identifier {}", name));
            vec![Instr::IMov(
                Val::Reg(Reg::RAX),
                Val::RegOffset(Reg::RSP, offset),
            )]
        }

        Expr::UnOp(op, sub) => {
            let mut instrs = compile_to_instrs(sub, si, env);
            match op {
                Op1::Add1 => instrs.push(Instr::IAdd(Val::Reg(Reg::RAX), Val::Imm(1))),
                Op1::Sub1 => instrs.push(Instr::ISub(Val::Reg(Reg::RAX), Val::Imm(1))),
            }
            instrs
        }

        Expr::BinOp(op, e1, e2) => {
            // left-to-right:
            // 1) compile e1 -> rax
            // 2) save rax to [rsp + offset(si)]
            // 3) compile e2 with si+1 -> rax
            // 4) combine using saved left operand
            let save_off = stack_offset(si);

            let mut instrs = compile_to_instrs(e1, si, env);
            instrs.push(Instr::IMov(
                Val::RegOffset(Reg::RSP, save_off),
                Val::Reg(Reg::RAX),
            ));

            instrs.extend(compile_to_instrs(e2, si + 1, env));

            match op {
                Op2::Plus => {
                    instrs.push(Instr::IAdd(
                        Val::Reg(Reg::RAX),
                        Val::RegOffset(Reg::RSP, save_off),
                    ));
                }
                Op2::Times => {
                    instrs.push(Instr::IMul(
                        Val::Reg(Reg::RAX),
                        Val::RegOffset(Reg::RSP, save_off),
                    ));
                }
                Op2::Minus => {
                    // compute left - right
                    // rax currently = right
                    // put right in temp slot si+1, load left into rax, then sub temp
                    let right_off = stack_offset(si + 1);
                    instrs.push(Instr::IMov(
                        Val::RegOffset(Reg::RSP, right_off),
                        Val::Reg(Reg::RAX),
                    ));
                    instrs.push(Instr::IMov(
                        Val::Reg(Reg::RAX),
                        Val::RegOffset(Reg::RSP, save_off),
                    ));
                    instrs.push(Instr::ISub(
                        Val::Reg(Reg::RAX),
                        Val::RegOffset(Reg::RSP, right_off),
                    ));
                }
            }

            instrs
        }

        Expr::Let(binds, body) => {
            // 1) check duplicates in THIS let
            // 2) compile each binding in order, storing at stack slot si, si+1, ...
            //    each binding visible to later bindings and body
            let mut seen: HashMap<String, ()> = HashMap::new();
            for (name, _) in binds.iter() {
                if seen.contains_key(name) {
                    panic!("Duplicate binding");
                }
                seen = seen.update(name.clone(), ());
            }

            let mut cur_env = env.clone();
            let mut cur_si = si;
            let mut instrs: Vec<Instr> = Vec::new();

            for (name, rhs) in binds.iter() {
                // compile rhs with current env
                instrs.extend(compile_to_instrs(rhs, cur_si, &cur_env));

                // store result to stack slot
                let off = stack_offset(cur_si);
                instrs.push(Instr::IMov(
                    Val::RegOffset(Reg::RSP, off),
                    Val::Reg(Reg::RAX),
                ));

                // extend env so next bindings/body can see it
                cur_env = cur_env.update(name.clone(), off);
                cur_si += 1;
            }

            instrs.extend(compile_to_instrs(body, cur_si, &cur_env));
            instrs
        }
    }
}

// ============= Code Generation =============

/// Convert a Val to its assembly string representation
fn val_to_str(v: &Val) -> String {
    match v {
        Val::Reg(Reg::RAX) => String::from("rax"),
        Val::Reg(Reg::RSP) => String::from("rsp"),
        Val::Imm(n) => format!("{}", n),
        Val::RegOffset(Reg::RSP, offset) => format!("[rsp + {}]", offset),
        Val::RegOffset(Reg::RAX, offset) => format!("[rax + {}]", offset),
    }
}

/// Convert an Instr to its assembly string representation
fn instr_to_str(i: &Instr) -> String {
    match i {
        Instr::IMov(dst, src) => format!("mov {}, {}", val_to_str(dst), val_to_str(src)),
        Instr::IAdd(dst, src) => format!("add {}, {}", val_to_str(dst), val_to_str(src)),
        Instr::ISub(dst, src) => format!("sub {}, {}", val_to_str(dst), val_to_str(src)),
        Instr::IMul(dst, src) => format!("imul {}, {}", val_to_str(dst), val_to_str(src)),
    }
}

/// Compile an expression to a complete assembly string
fn compile(e: &Expr) -> String {
    let env: HashMap<String, i32> = HashMap::new();
    let instrs = compile_to_instrs(e, 2, &env);
    instrs
        .iter()
        .map(|i| instr_to_str(i))
        .collect::<Vec<String>>()
        .join("\n  ")
}

// ============= Main =============

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input.snek> <output.s>", args[0]);
        std::process::exit(1);
    }

    let in_name = &args[1];
    let out_name = &args[2];

    // Read input file
    let mut in_file = File::open(in_name)?;
    let mut in_contents = String::new();
    in_file.read_to_string(&mut in_contents)?;

    // Parse S-expression from text
    let sexp = parse(&in_contents).unwrap_or_else(|_| panic!("Invalid"));

    // Convert S-expression to our AST
    let expr = parse_expr(&sexp);

    // Generate assembly instructions
    let instrs = compile(&expr);

    // Wrap instructions in assembly program template
    let asm_program = format!(
        "section .text
global our_code_starts_here
our_code_starts_here:
  {}
  ret
",
        instrs
    );

    // Write output assembly file
    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;

    Ok(())
}

// ============= TESTS =============

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(s: &str) -> Expr {
        parse_expr(&parse(s).unwrap())
    }

    #[test]
    fn test_parse_number() {
        let expr = parse_str("42");
        match expr {
            Expr::Number(42) => (),
            _ => panic!("Expected Number(42), got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_identifier() {
        let expr = parse_str("x");
        match expr {
            Expr::Id(name) => assert_eq!(name, "x"),
            _ => panic!("Expected Id(\"x\"), got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_add1() {
        let expr = parse_str("(add1 5)");
        match expr {
            Expr::UnOp(Op1::Add1, _) => (),
            _ => panic!("Expected UnOp(Add1, ...), got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_binary_plus() {
        let expr = parse_str("(+ 1 2)");
        match expr {
            Expr::BinOp(Op2::Plus, _, _) => (),
            _ => panic!("Expected BinOp(Plus, ...), got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_let_simple() {
        let expr = parse_str("(let ((x 5)) x)");
        match expr {
            Expr::Let(bindings, _) => {
                assert_eq!(bindings.len(), 1);
                assert_eq!(bindings[0].0, "x");
            }
            _ => panic!("Expected Let, got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_let_multiple_bindings() {
        let expr = parse_str("(let ((x 5) (y 6)) (+ x y))");
        match expr {
            Expr::Let(bindings, _) => {
                assert_eq!(bindings.len(), 2);
            }
            _ => panic!("Expected Let with 2 bindings, got {:?}", expr),
        }
    }

    #[test]
    #[should_panic(expected = "Duplicate binding")]
    fn test_duplicate_binding() {
        let expr = parse_str("(let ((x 1) (x 2)) x)");
        let env: HashMap<String, i32> = HashMap::new();
        compile_to_instrs(&expr, 2, &env);
    }

    #[test]
    #[should_panic(expected = "Unbound variable identifier y")]
    fn test_unbound_variable() {
        let expr = parse_str("y");
        let env: HashMap<String, i32> = HashMap::new();
        compile_to_instrs(&expr, 2, &env);
    }

    #[test]
    fn test_compile_number() {
        let expr = Expr::Number(42);
        let env: HashMap<String, i32> = HashMap::new();
        let instrs = compile_to_instrs(&expr, 2, &env);
        assert_eq!(instrs.len(), 1);
    }
}