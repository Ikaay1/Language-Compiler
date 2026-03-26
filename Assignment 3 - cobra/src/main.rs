use im::HashMap;
use sexp::Atom::*;
use sexp::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;

// ============= AST =============

#[derive(Debug)]
enum Op1 {
    Add1,
    Sub1,
    Negate,
    IsNum,
    IsBool,
}

#[derive(Debug)]
enum Op2 {
    Plus,
    Minus,
    Times,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    Equal,
}

#[derive(Debug)]
enum Expr {
    Num(i32),
    Bool(bool),
    Input,
    Var(String),
    Let(Vec<(String, Expr)>, Box<Expr>),
    UnOp(Op1, Box<Expr>),
    BinOp(Op2, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Block(Vec<Expr>),
    Loop(Box<Expr>),
    Break(Box<Expr>),
    Set(String, Box<Expr>),
}

// ============= Constants =============

const TRUE_VAL: i64 = 3;
const FALSE_VAL: i64 = 1;

const MAX_ENCODED: i64 = 2147483646;
const MIN_ENCODED: i64 = -2147483648;

fn encode_num(n: i32) -> i64 {
    (n as i64) << 1
}

fn stack_offset(si: i32) -> i32 {
    -8 * si
}

fn new_label(counter: &mut i32, prefix: &str) -> String {
    let current = *counter;
    *counter += 1;
    format!("{prefix}_{current}")
}

fn is_reserved(name: &str) -> bool {
    matches!(
        name,
        "let"
            | "add1"
            | "sub1"
            | "negate"
            | "true"
            | "false"
            | "input"
            | "if"
            | "block"
            | "loop"
            | "break"
            | "set!"
            | "isnum"
            | "isbool"
    )
}

// ============= Parsing =============

fn parse_expr(s: &Sexp) -> Expr {
    match s {
        Sexp::Atom(I(n)) => {
            let val = i32::try_from(*n).unwrap_or_else(|_| panic!("Invalid"));
            Expr::Num(val)
        }
        Sexp::Atom(S(name)) => match name.as_str() {
            "true" => Expr::Bool(true),
            "false" => Expr::Bool(false),
            "input" => Expr::Input,
            _ => {
                if is_reserved(name) {
                    panic!("Invalid");
                }
                Expr::Var(name.to_string())
            }
        },
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(op)), e] if op == "add1" => {
                Expr::UnOp(Op1::Add1, Box::new(parse_expr(e)))
            }
            [Sexp::Atom(S(op)), e] if op == "sub1" => {
                Expr::UnOp(Op1::Sub1, Box::new(parse_expr(e)))
            }
            [Sexp::Atom(S(op)), e] if op == "negate" => {
                Expr::UnOp(Op1::Negate, Box::new(parse_expr(e)))
            }
            [Sexp::Atom(S(op)), e] if op == "isnum" => {
                Expr::UnOp(Op1::IsNum, Box::new(parse_expr(e)))
            }
            [Sexp::Atom(S(op)), e] if op == "isbool" => {
                Expr::UnOp(Op1::IsBool, Box::new(parse_expr(e)))
            }

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
            [Sexp::Atom(S(op)), e1, e2] if op == "<" => Expr::BinOp(
                Op2::Less,
                Box::new(parse_expr(e1)),
                Box::new(parse_expr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == ">" => Expr::BinOp(
                Op2::Greater,
                Box::new(parse_expr(e1)),
                Box::new(parse_expr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "<=" => Expr::BinOp(
                Op2::LessEq,
                Box::new(parse_expr(e1)),
                Box::new(parse_expr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == ">=" => Expr::BinOp(
                Op2::GreaterEq,
                Box::new(parse_expr(e1)),
                Box::new(parse_expr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "=" => Expr::BinOp(
                Op2::Equal,
                Box::new(parse_expr(e1)),
                Box::new(parse_expr(e2)),
            ),

            [Sexp::Atom(S(op)), Sexp::List(bindings), body] if op == "let" => {
                if bindings.is_empty() {
                    panic!("Invalid");
                }
                let parsed_binds = bindings.iter().map(parse_bind).collect();
                Expr::Let(parsed_binds, Box::new(parse_expr(body)))
            }

            [Sexp::Atom(S(op)), cond, thn, els] if op == "if" => Expr::If(
                Box::new(parse_expr(cond)),
                Box::new(parse_expr(thn)),
                Box::new(parse_expr(els)),
            ),

            [Sexp::Atom(S(op)), exprs @ ..] if op == "block" => {
                if exprs.is_empty() {
                    panic!("Invalid");
                }
                Expr::Block(exprs.iter().map(parse_expr).collect())
            }

            [Sexp::Atom(S(op)), e] if op == "loop" => Expr::Loop(Box::new(parse_expr(e))),
            [Sexp::Atom(S(op)), e] if op == "break" => Expr::Break(Box::new(parse_expr(e))),

            [Sexp::Atom(S(op)), Sexp::Atom(S(name)), e] if op == "set!" => {
                if is_reserved(name) {
                    panic!("Invalid");
                }
                Expr::Set(name.to_string(), Box::new(parse_expr(e)))
            }

            _ => panic!("Invalid"),
        },
        _ => panic!("Invalid"),
    }
}

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

// ============= Assembly Helpers =============

fn bool_result_from_jump(jump_instr: &str, counter: &mut i32) -> String {
    let true_label = new_label(counter, "bool_true");
    let end_label = new_label(counter, "bool_end");
    format!(
        "
  {jump_instr} {true_label}
  mov rax, {FALSE_VAL}
  jmp {end_label}
{true_label}:
  mov rax, {TRUE_VAL}
{end_label}:
"
    )
}

// ============= Compilation =============

fn overflow_check() -> String {
    format!(
        "
  cmp rax, {}
  jg throw_overflow_error
  cmp rax, {}
  jl throw_overflow_error",
        MAX_ENCODED, MIN_ENCODED
    )
}

fn compile_expr(
    e: &Expr,
    si: i32,
    env: &HashMap<String, i32>,
    label_counter: &mut i32,
    break_target: Option<&str>,
) -> String {
    match e {
        Expr::Num(n) => format!("mov rax, {}", encode_num(*n)),

        Expr::Bool(true) => format!("mov rax, {}", TRUE_VAL),
        Expr::Bool(false) => format!("mov rax, {}", FALSE_VAL),

        Expr::Input => "mov rax, rdi".to_string(),

        Expr::Var(name) => {
            let offset = env
                .get(name)
                .unwrap_or_else(|| panic!("Unbound variable identifier {}", name));
            format!("mov rax, [rsp + {}]", offset)
        }

        Expr::UnOp(op, subexpr) => {
            let compiled = compile_expr(subexpr, si, env, label_counter, break_target);
            match op {
                Op1::Add1 => format!(
    "{compiled}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  add rax, 2{}",
    overflow_check()
),
                Op1::Sub1 => format!(
    "{compiled}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  sub rax, 2{}",
    overflow_check()
),
                Op1::Negate => format!(
    "{compiled}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  neg rax{}",
    overflow_check()
),
                Op1::IsNum => {
                    let true_label = new_label(label_counter, "isnum_true");
                    let end_label = new_label(label_counter, "isnum_end");
                    format!(
                        "{compiled}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  je {true_label}
  mov rax, {FALSE_VAL}
  jmp {end_label}
{true_label}:
  mov rax, {TRUE_VAL}
{end_label}:"
                    )
                }
                Op1::IsBool => {
                    let true_label = new_label(label_counter, "isbool_true");
                    let end_label = new_label(label_counter, "isbool_end");
                    format!(
                        "{compiled}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 1
  je {true_label}
  mov rax, {FALSE_VAL}
  jmp {end_label}
{true_label}:
  mov rax, {TRUE_VAL}
{end_label}:"
                    )
                }
            }
        }

        Expr::BinOp(op, left, right) => {
            let left_off = stack_offset(si);
            let right_off = stack_offset(si + 1);

            let left_code = compile_expr(left, si, env, label_counter, break_target);
            let right_code = compile_expr(right, si + 1, env, label_counter, break_target);

            match op {
                Op2::Plus => format!(
                    "{left_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rsp + {left_off}], rax
{right_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  add rax, [rsp + {left_off}]{}",
                    overflow_check()
                ),

                Op2::Minus => format!(
                    "{left_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rsp + {left_off}], rax
{right_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rsp + {right_off}], rax
  mov rax, [rsp + {left_off}]
  sub rax, [rsp + {right_off}]{}",
                    overflow_check()
                ),

                Op2::Times => format!(
                    "{left_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rsp + {left_off}], rax
{right_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  imul rax, [rsp + {left_off}]
  sar rax, 1{}",
                    overflow_check()
                ),

                Op2::Less | Op2::Greater | Op2::LessEq | Op2::GreaterEq => {
                    let jump = match op {
                        Op2::Less => "jl",
                        Op2::Greater => "jg",
                        Op2::LessEq => "jle",
                        Op2::GreaterEq => "jge",
                        _ => unreachable!(),
                    };

                    let compare_result = bool_result_from_jump(jump, label_counter);

                    format!(
                        "{left_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rsp + {left_off}], rax
{right_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  cmp [rsp + {left_off}], rax
{compare_result}"
                    )
                }

                Op2::Equal => {
                    let eq_true = new_label(label_counter, "eq_true");
                    let eq_end = new_label(label_counter, "eq_end");
                    format!(
                        "{left_code}
  mov [rsp + {left_off}], rax
{right_code}
  mov [rsp + {right_off}], rax
  mov rbx, [rsp + {left_off}]
  and rbx, 1
  mov rcx, [rsp + {right_off}]
  and rcx, 1
  cmp rbx, rcx
  jne throw_invalid_arg
  mov rax, [rsp + {left_off}]
  cmp rax, [rsp + {right_off}]
  je {eq_true}
  mov rax, {FALSE_VAL}
  jmp {eq_end}
{eq_true}:
  mov rax, {TRUE_VAL}
{eq_end}:"
                    )
                }
            }
        }

        Expr::Let(bindings, body) => {
            let mut seen: HashMap<String, ()> = HashMap::new();
            let mut curr_env = env.clone();
            let mut curr_si = si;
            let mut parts: Vec<String> = Vec::new();

            for (name, expr) in bindings {
                if seen.contains_key(name) {
                    panic!("Duplicate binding");
                }
                seen = seen.update(name.clone(), ());
                let expr_code = compile_expr(expr, curr_si, &curr_env, label_counter, break_target);
                let off = stack_offset(curr_si);
                parts.push(format!(
                    "{expr_code}
  mov [rsp + {off}], rax"
                ));
                curr_env = curr_env.update(name.clone(), off);
                curr_si += 1;
            }

            parts.push(compile_expr(
                body,
                curr_si,
                &curr_env,
                label_counter,
                break_target,
            ));

            parts.join("\n")
        }

        Expr::If(cond, thn, els) => {
            let else_label = new_label(label_counter, "if_else");
            let end_label = new_label(label_counter, "if_end");
            let cond_code = compile_expr(cond, si, env, label_counter, break_target);
            let thn_code = compile_expr(thn, si, env, label_counter, break_target);
            let els_code = compile_expr(els, si, env, label_counter, break_target);

            format!(
                "{cond_code}
  cmp rax, {FALSE_VAL}
  je {else_label}
{thn_code}
  jmp {end_label}
{else_label}:
{els_code}
{end_label}:"
            )
        }

        Expr::Block(exprs) => exprs
            .iter()
            .map(|expr| compile_expr(expr, si, env, label_counter, break_target))
            .collect::<Vec<_>>()
            .join("\n"),

        Expr::Loop(body) => {
            let loop_start = new_label(label_counter, "loop_start");
            let loop_end = new_label(label_counter, "loop_end");
            let body_code = compile_expr(body, si, env, label_counter, Some(&loop_end));

            format!(
                "{loop_start}:
{body_code}
  jmp {loop_start}
{loop_end}:"
            )
        }

        Expr::Break(expr) => match break_target {
            Some(target) => {
                let value_code = compile_expr(expr, si, env, label_counter, break_target);
                format!(
                    "{value_code}
  jmp {target}"
                )
            }
            None => panic!("break outside of loop"),
        },

        Expr::Set(name, expr) => {
            let off = env
                .get(name)
                .unwrap_or_else(|| panic!("Unbound variable identifier {}", name));
            let value_code = compile_expr(expr, si, env, label_counter, break_target);
            format!(
                "{value_code}
  mov [rsp + {off}], rax"
            )
        }
    }
}

fn compile(e: &Expr) -> String {
    let env: HashMap<String, i32> = HashMap::new();
    let mut label_counter = 0;
    let body = compile_expr(e, 2, &env, &mut label_counter, None);

    format!(
        "
section .text
extern _snek_error
global our_code_starts_here

throw_invalid_arg:
  mov rdi, 1
  push rsp
  call _snek_error

throw_overflow_error:
  mov rdi, 2
  push rsp
  call _snek_error

our_code_starts_here:
  {}
  ret
",
        body
    )
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

    let mut in_file = File::open(in_name)?;
    let mut in_contents = String::new();
    in_file.read_to_string(&mut in_contents)?;

    let sexp = parse(&in_contents).unwrap_or_else(|_| panic!("Invalid"));
    let expr = parse_expr(&sexp);
    let asm_program = compile(&expr);

    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;
    Ok(())
}