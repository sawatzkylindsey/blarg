use blarg::{prelude::*, CommandLineParser, Condition, Parameter, Scalar};
use std::env;

fn main() {
    let contains_dynamic_x = env::var("DYNAMIC_X").is_ok();
    let contains_dynamic_y = env::var("DYNAMIC_Y").is_ok();

    let mut sub: u32 = 0;
    let mut arg_0: bool = false;
    let mut arg_1: bool = false;
    let mut arg_2: bool = false;

    let mut condition = Condition::new(Scalar::new(&mut sub), "sub")
        // "0" is an undocumented sub-command, but will only available when environment contains `DYNAMIC_X`.
        // "1" is a regular sub-command.
        .choice(1, "the one sub-command");

    if contains_dynamic_y {
        // "2" is a sub-command that will only be available when the environment contains `DYNAMIC_Y`.
        condition = condition.choice(2, "the two sub-command");
    }

    let clp = CommandLineParser::new("sub-command");
    let mut clp = clp.branch(condition).command(1, |sub_command| {
        sub_command.add(Parameter::argument(Scalar::new(&mut arg_1), "arg"))
    });

    if contains_dynamic_x {
        clp = clp.command(0, |sub_command| {
            sub_command.add(Parameter::argument(Scalar::new(&mut arg_0), "arg"))
        });
    }

    if contains_dynamic_y {
        clp = clp.command(2, |sub_command| {
            sub_command.add(Parameter::argument(Scalar::new(&mut arg_2), "arg"))
        });
    }

    let parser = clp.build();

    parser.parse();

    println!("Used sub-command '{sub}'.");
    match sub {
        0 => {
            println!("arg_0: {arg_0}");
            assert!(!arg_1);
            assert!(!arg_2);
        }
        1 => {
            assert!(!arg_0);
            println!("arg_1: {arg_1}");
            assert!(!arg_2);
        }
        2 => {
            assert!(!arg_0);
            assert!(!arg_1);
            println!("arg_2: {arg_2}");
        }
        _ => {
            panic!("impossible - the parser will reject any variants not specified via `add(..)`.")
        }
    }
}
