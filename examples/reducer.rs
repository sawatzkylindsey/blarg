use blarg::field::{Container, Field, Switch, Value};
use blarg::parser::{ArgumentParser, Parameter};
use std::collections::HashSet;
use std::hash::Hash;
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

#[derive(Debug, PartialEq, Eq, Hash)]
enum Country {
    Canada,
    Pakistan,
}

impl FromStr for Country {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "canada" => Ok(Country::Canada),
            "pakistan" => Ok(Country::Pakistan),
            _ => Err(format!("unknown: {}", value)),
        }
    }
}

fn main() {
    let mut verbose: bool = false;
    let mut operand: Operand = Operand::Add;
    let mut initial: Option<u32> = None;
    let mut countries: HashSet<Country> = HashSet::default();
    let mut items: Vec<u32> = Vec::default();

    let ap = ArgumentParser::new("reducer");
    let parser = ap
        .add(
            Parameter::option("verbose", Some('v')).help("do dee doo"),
            Field::binding(Switch::new(&mut verbose, true)),
        )
        .add(
            Parameter::option("operand", Some('o')),
            Field::binding(Value::new(&mut operand)),
        )
        .add(
            Parameter::option("initial", None),
            Field::binding(Container::new(&mut initial)),
        )
        .add(
            Parameter::option("countries", None),
            Field::binding(Container::new(&mut countries)),
        )
        .add(
            Parameter::argument("items").help("the items todo"),
            Field::binding(Container::new(&mut items)),
        )
        .build()
        .expect("Invalid argument parser configuration");
    parser.parse();
    println!("Items: {items:?}");
    execute(verbose, operand, initial, countries, items);
}

fn execute(
    verbose: bool,
    operand: Operand,
    initial: Option<u32>,
    countries: HashSet<Country>,
    items: Vec<u32>,
) {
    let result: u32 = items
        .iter()
        .fold(initial.unwrap_or(0), |a, b| match operand {
            Operand::Add => a + b,
            Operand::Multiply => a * b,
        });
    println!("Result: {result}");
}
