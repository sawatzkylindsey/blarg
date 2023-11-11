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
