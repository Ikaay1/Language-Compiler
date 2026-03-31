#[link(name = "our_code")]
extern "C" {
    #[link_name = "\x01our_code_starts_here"]
    fn our_code_starts_here(input: i64) -> i64;
}

const TRUE_VAL: i64 = 3;
const FALSE_VAL: i64 = 1;

fn parse_input(input: &str) -> i64 {
    match input {
        "true" => TRUE_VAL,
        "false" => FALSE_VAL,
        _ => {
            let n: i64 = input.parse().expect("invalid input");
            n << 1
        }
    }
}

fn print_value(v: i64) {
    if v == TRUE_VAL {
        println!("true");
    } else if v == FALSE_VAL {
        println!("false");
    } else if v & 1 == 0 {
        println!("{}", v >> 1);
    } else {
        println!("Unknown value: {}", v);
    }
}

#[no_mangle]
pub extern "C" fn snek_print(val: i64) -> i64 {
    print_value(val);
    val
}

#[no_mangle]
pub extern "C" fn snek_error(errcode: i64) -> ! {
    match errcode {
        1 => eprintln!("invalid argument"),
        2 => eprintln!("overflow"),
        _ => eprintln!("an error occurred {}", errcode),
    }
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input = if args.len() > 1 {
        parse_input(&args[1])
    } else {
        FALSE_VAL
    };

    let result = unsafe { our_code_starts_here(input) };
    print_value(result);
}