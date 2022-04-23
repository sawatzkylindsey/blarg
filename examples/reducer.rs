use blarg::field::{Container, Field, Value};
use blarg::parser::ArgumentParser;
use std::str::FromStr;

#[derive(Debug)]
enum Operand {
    Add,
    Multiply,
}

impl FromStr for Operand {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "add" => Ok(Operand::Add),
            "multiply" => Ok(Operand::Multiply),
            _ => Err(format!("unknown: {}", value)),
        }
    }
}

fn main() {
    let mut verbose: bool = false;
    let mut operand: Operand = Operand::Add;
    let mut initial: Option<u32> = None;
    let mut items: Vec<u32> = Vec::new();

    let mut ap = ArgumentParser::new("reducer");
    println!("{:?}", ap);
    ap = ap
        .add_option(Field::binding(Value::new(&mut verbose)))
        .add_option(Field::binding(Value::new(&mut operand)))
        .add_option(Field::binding(Container::new(&mut initial)))
        .add_option(Field::binding(Container::new(&mut items)));
    ap.parse();
}
