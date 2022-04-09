use std::str::FromStr;
use blarg::parser::{ArgumentParser};

#[derive(Debug)]
enum Operand {
    Add,
    Multiply,
}

impl FromStr for Operand {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase() {
            "add" => Ok(Operand::Add),
            "multiply" => Ok(Operand::Multiply),
            _ => Err(format!("unknown: {}", value))
        }
    }
}

fn main() {
    let verbose: bool = false;
    let operand: Operand = Operand::Add;
    let initial: Option<u32> = None;
    let items: Vec<u32> = Vec::new();

    let mut ap = ArgumentParser::new();
    ap.add_option(Field::builder()
        .reference(FieldReference::new(&mut verbose))
        .build(),
    );
    ap.add_option(Field::builder()
        .reference(FieldReference::new(&mut operand))
        .build(),
    );
    ap.add_option(Field::builder()
        .reference(FieldReference::new(&mut initial))
        .build(),
    );
    ap.add_argument(
        Field::builder()
            .reference(FieldReferenceCollection::new(&mut items))
            .build(),
    );
    ap.parse();
}

