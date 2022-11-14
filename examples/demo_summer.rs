use blarg::{Collection, CommandParser, Nargs, Parameter};

fn main() {
    let mut items: Vec<u32> = Vec::default();

    let cp = CommandParser::new("summer");
    let parser = cp
        .add(
            Parameter::argument(Collection::new(&mut items, Nargs::AtLeastOne), "item")
                .help("The items to sum."),
        )
        .build()
        .expect("Invalid argument parser configuration");

    parser.parse();
    let sum: u32 = items.iter().sum();
    println!("Sum: {sum}");
}
