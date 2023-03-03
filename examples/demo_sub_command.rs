use blarg::{CommandLineParser, Condition, Parameter, Scalar, Switch};

fn main() {
    let mut sub: u32 = 0;
    let mut arg_0: bool = false;
    let mut opt_0: bool = false;
    let mut arg_1: bool = false;

    let clp = CommandLineParser::new("sub-command");
    let parser = clp
        .branch(
            Condition::new(Scalar::new(&mut sub), "sub")
                // "0" is an undocumented sub-command.
                // "1" is a regular sub-command.
                .choice(1, "the one sub-command")
                // "2" is a false sub-command.
                // It will appear in the documentation, but only those specified via `add(..)` actually affect the program structure.
                .choice(2, "the two sub-command"),
        )
        .add(0, Parameter::argument(Scalar::new(&mut arg_0), "arg"))
        .add(
            0,
            Parameter::option(Switch::new(&mut opt_0, true), "opt", None),
        )
        .add(1, Parameter::argument(Scalar::new(&mut arg_1), "arg"))
        // Since we never add "2", it isn't a true sub-command.
        .build()
        .expect("The parser configuration must be valid (ex: no parameter name repeats).");

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
        _ => {
            panic!("impossible - the parser will reject any variants not specified via `add(..)`.")
        }
    }
}
