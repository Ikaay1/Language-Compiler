use im::HashMap;
use sexp::Atom::*;
use sexp::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;

// ============= Program AST =============

#[derive(Debug)]
struct Program {
    defns: Vec<Definition>,
    main: Expr,
}

#[derive(Debug)]
struct Definition {
    name: String,
    params: Vec<String>,
    body: Expr,
}

// ============= Expr AST =============

#[derive(Debug)]
enum Op1 {
    Add1,
    Sub1,
    Negate,
    IsNum,
    IsBool,
    Print,
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
    Call(String, Vec<Expr>),
}

// ============= Environment =============

#[derive(Debug, Clone)]
enum Loc {
    Local(i32),  // stack slot index: 1 => [rbp - 8], 2 => [rbp - 16], ...
    Param(i32),  // positive rbp offset: 16, 24, 32, ...
}

// ============= Constants =============

const TRUE_VAL: i64 = 3;
const FALSE_VAL: i64 = 1;

const MAX_ENCODED: i64 = 2147483646;
const MIN_ENCODED: i64 = -2147483648;

fn encode_num(n: i32) -> i64 {
    (n as i64) << 1
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
            | "fun"
            | "add1"
            | "sub1"
            | "negate"
            | "isnum"
            | "isbool"
            | "print"
            | "true"
            | "false"
            | "input"
            | "if"
            | "block"
            | "loop"
            | "break"
            | "set!"
    )
}

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
{end_label}:"
    )
}

// ============= Parsing =============

fn parse_program(s: &Sexp) -> Program {
    match s {
        Sexp::List(items) => {
            let mut defns = vec![];
            let mut main_expr: Option<Expr> = None;

            for item in items {
                if let Some(defn) = try_parse_defn(item) {
                    defns.push(defn);
                } else if main_expr.is_none() {
                    main_expr = Some(parse_expr(item));
                } else {
                    panic!("Multiple main expressions");
                }
            }

            Program {
                defns,
                main: main_expr.unwrap_or_else(|| panic!("No main expression")),
            }
        }
        _ => panic!("Invalid program"),
    }
}

fn try_parse_defn(s: &Sexp) -> Option<Definition> {
    match s {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(fun_kw)), Sexp::List(signature), body] if fun_kw == "fun" => {
                match &signature[..] {
                    [Sexp::Atom(S(name)), params @ ..] => {
                        if is_reserved(name) {
                            panic!("Invalid function name");
                        }

                        let mut parsed_params = vec![];
                        for p in params {
                            match p {
                                Sexp::Atom(S(param)) => {
                                    if is_reserved(param) {
                                        panic!("Invalid parameter");
                                    }
                                    if parsed_params.contains(param) {
                                        panic!("Duplicate parameter");
                                    }
                                    parsed_params.push(param.clone());
                                }
                                _ => panic!("Invalid parameter"),
                            }
                        }

                        Some(Definition {
                            name: name.clone(),
                            params: parsed_params,
                            body: parse_expr(body),
                        })
                    }
                    _ => panic!("Invalid function signature"),
                }
            }
            _ => None,
        },
        _ => None,
    }
}

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
            [Sexp::Atom(S(op)), e] if op == "print" => {
                Expr::UnOp(Op1::Print, Box::new(parse_expr(e)))
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

            [Sexp::Atom(S(name)), args @ ..] => {
                if is_reserved(name) {
                    panic!("Invalid");
                }
                Expr::Call(name.clone(), args.iter().map(parse_expr).collect())
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

// ============= Stack Depth =============

fn max_stack_expr(e: &Expr, si: i32) -> i32 {
    match e {
        Expr::Num(_) | Expr::Bool(_) | Expr::Input | Expr::Var(_) => si - 1,

        Expr::UnOp(_, sub) => max_stack_expr(sub, si),

        Expr::BinOp(_, e1, e2) => {
            let m1 = max_stack_expr(e1, si);
            let m2 = max_stack_expr(e2, si + 1);
            m1.max(m2).max(si + 1)
        }

        Expr::Let(bindings, body) => {
            let mut cur_si = si;
            let mut max_seen = si - 1;

            for (_, rhs) in bindings {
                max_seen = max_seen.max(max_stack_expr(rhs, cur_si));
                max_seen = max_seen.max(cur_si);
                cur_si += 1;
            }

            max_seen.max(max_stack_expr(body, cur_si))
        }

        Expr::If(c, t, e) => {
            let mc = max_stack_expr(c, si);
            let mt = max_stack_expr(t, si);
            let me = max_stack_expr(e, si);
            mc.max(mt).max(me)
        }

        Expr::Block(exprs) => exprs
            .iter()
            .map(|expr| max_stack_expr(expr, si))
            .max()
            .unwrap_or(si - 1),

        Expr::Loop(body) | Expr::Break(body) | Expr::Set(_, body) => max_stack_expr(body, si),

        Expr::Call(_, args) => args
            .iter()
            .map(|arg| max_stack_expr(arg, si))
            .max()
            .unwrap_or(si - 1),
    }
}

fn stack_space_for_expr(e: &Expr) -> i32 {
    8 * max_stack_expr(e, 1)
}

// ============= Compilation =============

fn compile_expr(
    e: &Expr,
    si: i32,
    env: &HashMap<String, Loc>,
    funs: &HashMap<String, usize>,
    label_counter: &mut i32,
    break_target: Option<&str>,
) -> String {
    match e {
        Expr::Num(n) => format!("mov rax, {}", encode_num(*n)),

        Expr::Bool(true) => format!("mov rax, {}", TRUE_VAL),
        Expr::Bool(false) => format!("mov rax, {}", FALSE_VAL),

        Expr::Input => "mov rax, r12".to_string(),

        Expr::Var(name) => match env.get(name) {
            Some(Loc::Local(slot)) => format!("mov rax, [rbp - {}]", slot * 8),
            Some(Loc::Param(off)) => format!("mov rax, [rbp + {}]", off),
            None => panic!("Unbound variable identifier {}", name),
        },

        Expr::UnOp(op, subexpr) => {
            let compiled = compile_expr(subexpr, si, env, funs, label_counter, break_target);
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
                Op1::Print => format!(
                    "{compiled}
  mov rdi, rax
  call _snek_print"
                ),
            }
        }

        Expr::BinOp(op, left, right) => {
            let left_slot = si;
            let right_slot = si + 1;

            let left_code = compile_expr(left, si, env, funs, label_counter, break_target);
            let right_code = compile_expr(right, si + 1, env, funs, label_counter, break_target);

            match op {
                Op2::Plus => format!(
                    "{left_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rbp - {}], rax
{right_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  add rax, [rbp - {}]{}",
                    left_slot * 8,
                    left_slot * 8,
                    overflow_check()
                ),

                Op2::Minus => format!(
                    "{left_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rbp - {}], rax
{right_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rbp - {}], rax
  mov rax, [rbp - {}]
  sub rax, [rbp - {}]{}",
                    left_slot * 8,
                    right_slot * 8,
                    left_slot * 8,
                    right_slot * 8,
                    overflow_check()
                ),

                Op2::Times => format!(
                    "{left_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  mov [rbp - {}], rax
{right_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  imul rax, [rbp - {}]
  sar rax, 1{}",
                    left_slot * 8,
                    left_slot * 8,
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
  mov [rbp - {}], rax
{right_code}
  mov rbx, rax
  and rbx, 1
  cmp rbx, 0
  jne throw_invalid_arg
  cmp [rbp - {}], rax
{compare_result}",
                        left_slot * 8,
                        left_slot * 8,
                    )
                }

                Op2::Equal => {
                    let eq_true = new_label(label_counter, "eq_true");
                    let eq_end = new_label(label_counter, "eq_end");
                    format!(
                        "{left_code}
  mov [rbp - {}], rax
{right_code}
  mov [rbp - {}], rax
  mov rbx, [rbp - {}]
  and rbx, 1
  mov rcx, [rbp - {}]
  and rcx, 1
  cmp rbx, rcx
  jne throw_invalid_arg
  mov rax, [rbp - {}]
  cmp rax, [rbp - {}]
  je {eq_true}
  mov rax, {FALSE_VAL}
  jmp {eq_end}
{eq_true}:
  mov rax, {TRUE_VAL}
{eq_end}:",
                        left_slot * 8,
                        right_slot * 8,
                        left_slot * 8,
                        right_slot * 8,
                        left_slot * 8,
                        right_slot * 8,
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
                let expr_code =
                    compile_expr(expr, curr_si, &curr_env, funs, label_counter, break_target);
                parts.push(format!(
                    "{expr_code}
  mov [rbp - {}], rax",
                    curr_si * 8
                ));
                curr_env = curr_env.update(name.clone(), Loc::Local(curr_si));
                curr_si += 1;
            }

            parts.push(compile_expr(
                body,
                curr_si,
                &curr_env,
                funs,
                label_counter,
                break_target,
            ));

            parts.join("\n")
        }

        Expr::If(cond, thn, els) => {
            let else_label = new_label(label_counter, "if_else");
            let end_label = new_label(label_counter, "if_end");
            let cond_code = compile_expr(cond, si, env, funs, label_counter, break_target);
            let thn_code = compile_expr(thn, si, env, funs, label_counter, break_target);
            let els_code = compile_expr(els, si, env, funs, label_counter, break_target);

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
            .map(|expr| compile_expr(expr, si, env, funs, label_counter, break_target))
            .collect::<Vec<_>>()
            .join("\n"),

        Expr::Loop(body) => {
            let loop_start = new_label(label_counter, "loop_start");
            let loop_end = new_label(label_counter, "loop_end");
            let body_code = compile_expr(body, si, env, funs, label_counter, Some(&loop_end));

            format!(
                "{loop_start}:
{body_code}
  jmp {loop_start}
{loop_end}:"
            )
        }

        Expr::Break(expr) => match break_target {
            Some(target) => {
                let value_code = compile_expr(expr, si, env, funs, label_counter, break_target);
                format!(
                    "{value_code}
  jmp {target}"
                )
            }
            None => panic!("break outside of loop"),
        },

        Expr::Set(name, expr) => {
            let value_code = compile_expr(expr, si, env, funs, label_counter, break_target);
            match env.get(name) {
                Some(Loc::Local(slot)) => format!(
                    "{value_code}
  mov [rbp - {}], rax",
                    slot * 8
                ),
                Some(Loc::Param(off)) => format!(
                    "{value_code}
  mov [rbp + {}], rax",
                    off
                ),
                None => panic!("Unbound variable identifier {}", name),
            }
        }

        Expr::Call(name, args) => {
            let expected = funs
                .get(name)
                .unwrap_or_else(|| panic!("Undefined function {}", name));

            if args.len() != *expected {
                panic!("Wrong number of arguments");
            }

            let mut parts = vec![];

            for arg in args.iter().rev() {
                parts.push(compile_expr(arg, si, env, funs, label_counter, break_target));
                parts.push("push rax".to_string());
            }

            parts.push(format!("call fun_{}", name));

            if !args.is_empty() {
                parts.push(format!("add rsp, {}", args.len() * 8));
            }

            parts.join("\n")
        }
    }
}

fn compile_defn(
    defn: &Definition,
    funs: &HashMap<String, usize>,
    label_counter: &mut i32,
) -> String {
    let mut env: HashMap<String, Loc> = HashMap::new();

    for (i, param) in defn.params.iter().enumerate() {
        let off = 16 + (i as i32) * 8;
        env = env.update(param.clone(), Loc::Param(off));
    }

    let stack_space = stack_space_for_expr(&defn.body);
    let body = compile_expr(&defn.body, 1, &env, funs, label_counter, None);

    format!(
        "fun_{}:
  push rbp
  mov rbp, rsp
  sub rsp, {}
  {}
  add rsp, {}
  pop rbp
  ret",
        defn.name, stack_space, body, stack_space
    )
}

fn compile_program(prog: &Program) -> String {
    let mut funs: HashMap<String, usize> = HashMap::new();

    for defn in &prog.defns {
        if funs.contains_key(&defn.name) {
            panic!("Duplicate function");
        }
        funs = funs.update(defn.name.clone(), defn.params.len());
    }

    let mut label_counter = 0;
    let mut pieces = vec![
        "section .text".to_string(),
        "extern _snek_error".to_string(),
        "extern _snek_print".to_string(),
        "global our_code_starts_here".to_string(),
        "".to_string(),
        "throw_invalid_arg:".to_string(),
        "  mov rdi, 1".to_string(),
        "  call _snek_error".to_string(),
        "".to_string(),
        "throw_overflow_error:".to_string(),
        "  mov rdi, 2".to_string(),
        "  call _snek_error".to_string(),
        "".to_string(),
    ];

    for defn in &prog.defns {
        pieces.push(compile_defn(defn, &funs, &mut label_counter));
        pieces.push(String::new());
    }

    let main_stack = stack_space_for_expr(&prog.main);
    let main_body = compile_expr(&prog.main, 1, &HashMap::new(), &funs, &mut label_counter, None);

    pieces.push("our_code_starts_here:".to_string());
    pieces.push("  push rbp".to_string());
    pieces.push("  mov rbp, rsp".to_string());
    pieces.push("  mov r12, rdi".to_string());
    pieces.push(format!("  sub rsp, {}", main_stack));
    pieces.push(format!("  {}", main_body.replace('\n', "\n  ")));
    pieces.push(format!("  add rsp, {}", main_stack));
    pieces.push("  pop rbp".to_string());
    pieces.push("  ret".to_string());

    pieces.join("\n")
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
    let prog = parse_program(&sexp);
    let asm_program = compile_program(&prog);

    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;
    Ok(())
}