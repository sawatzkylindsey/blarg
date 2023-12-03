use blarg::{prelude::*, CommandLineParser, Condition, Parameter, Scalar, Switch};

fn main() {
    let mut sub: u32 = 0;
    let mut arg_0: bool = false;
    let mut opt_0: bool = false;
    let mut arg_1: bool = false;

    let clp = CommandLineParser::new("sub-command");
    let parser = clp
        .about("Describe the base command line parser.  Let's make it a little long for fun.")
        .branch(
            Condition::new(Scalar::new(&mut sub), "sub")
                // "0" is an undocumented sub-command.
                // "1" is a regular sub-command.
                .choice(1, "the one sub-command")
                // "2" is a regular sub-command.
                .choice(2, "the two sub-command")
                // "3" is a false sub-command.
                // It will appear in the documentation, but only those specified via `command(..)` actually affect the program structure.
                .choice(3, "the three sub-command"),
        )
        .command(0, |sub_command| {
            sub_command
                .about("Describe the 0 sub-command parser.  Let's make it a little long for fun.")
                .add(Parameter::argument(Scalar::new(&mut arg_0), "arg"))
                .add(Parameter::option(
                    Switch::new(&mut opt_0, true),
                    "opt",
                    None,
                ))
        })
        .command(1, |sub_command| {
            sub_command
                .about("Describe the 1 sub-command parser.")
                .add(Parameter::argument(Scalar::new(&mut arg_1), "arg"))
        })
        // Specify an argument-less & option-less sub-command by leaving the 'sub' untouched.
        .command(2, |sub_command| sub_command)
        // Since we never add "3", it isn't a true sub-command.
        .build();

    parser.parse();

    println!("Used sub-command '{sub}'.");
    match sub {
        0 => {
            println!("arg_0: {arg_0}");
            println!("opt_0: {opt_0}");
            assert!(!arg_1);
        }
        1 => {
            assert!(!arg_0);
            assert!(!opt_0);
            println!("arg_1: {arg_1}");
        }
        2 => {
            assert!(!arg_0);
            assert!(!opt_0);
            assert!(!arg_1);
            println!("argument-less & option-less");
        }
        _ => {
            panic!(
                "impossible - the parser will reject any variants not specified via `command(..)`."
            )
        }
    }
}
