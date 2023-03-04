use blarg::CommandLineParser;

fn main() {
    let ap = CommandLineParser::new("empty");
    let parser = ap.build().expect("Invalid argument parser configuration");
    parser.parse();
    println!("empty parser");
}
