# blarg
`blarg` is a command line argument parser for Rust.
In brief, it provides type-safe, domain sensitive, argument/option paradigm command line parser functionality.
Check out the rustdoc for more information.

"blarg" is derived from the following:
* **arg** for argument parser.
* **bl**arg because all the other names are already taken 🤪.

### Derive Example

    use blarg::{derive::*, CommandLineParser, Parameter, Scalar, Switch};
    
    #[derive(Debug, Default, BlargParser)]
    #[blarg(program = "example")]
    struct Parameters {
        #[blarg(short = 'v')]
        verbose: bool,
        value: u32,
    }
    
    fn main() {
        let parameters = Parameters::blarg_parse();
        println!(
            "value: {}, verbose: {}",
            parameters.value, parameters.verbose
        );
    }

    $ ./main -h
    usage: example [-h] [-v] VALUE
    
    positional arguments:
     VALUE                                              type: u32
    
    options:
     -h, --help     Show this help message and exit.
     -v, --verbose

### Builder Example

    use blarg::{CommandLineParser, Parameter, Scalar, Switch};
    
    fn main() {
        let mut verbose: bool = false;
        let mut value: u32 = 0;
        let ap = CommandLineParser::new("example");
        let parser = ap
            .add(Parameter::option(
                Switch::new(&mut verbose, true),
                "verbose",
                Some('v'),
            ))
            .add(Parameter::argument(Scalar::new(&mut value), "value").meta(vec!["type: u32"]))
            .build()
            .expect("Invalid argument parser configuration");
        parser.parse();
        println!("value: {value}, verbose: {verbose}");
    }

    $ ./main -h
    usage: example [-h] [-v] VALUE
    
    positional arguments:
     VALUE
    
    options:
     -h, --help     Show this help message and exit.
     -v, --verbose

### Development

    cargo build --workspace
    cargo test --workspace
    cargo doc --open --no-deps --all-features --package blarg

    ./target/debug/examples/reducer -h
