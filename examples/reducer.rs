use blarg::{
    prelude::*, Collection, CommandLineParser, Nargs, Optional, Parameter, Scalar, Switch,
};
use std::collections::HashSet;
use std::hash::Hash;
use std::str::FromStr;

#[derive(Debug)]
enum Operand {
    Add,
    Multiply,
}

impl std::fmt::Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Add => write!(f, "add"),
            Operand::Multiply => write!(f, "multiply"),
        }
    }
}

impl FromStr for Operand {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "add" | "+" => Ok(Operand::Add),
            "multiply" | "*" => Ok(Operand::Multiply),
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
    let mut _verbose: bool = false;
    let mut operand: Operand = Operand::Add;
    let mut initial: Option<u32> = None;
    let mut _countries: HashSet<Country> = HashSet::default();
    let mut items: Vec<u32> = Vec::default();

    let ap = CommandLineParser::new("reducer");
    let parser = ap
        .add(
            Parameter::option(Switch::new(&mut _verbose, true), "verbose", Some('v'))
                .help("Do dee doo.  We're really stretching here HAAAAAAAA HAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA!"),
        )
        .add(
            Parameter::option(Scalar::new(&mut operand), "operand", Some('o'))
                .help("moot")
                .choice(Operand::Add, "+")
                .choice(Operand::Multiply, "*"),
        )
        .add(Parameter::option(
            Optional::new(&mut initial),
            "initial",
            None,
        ))
        .add(Parameter::option(
            Collection::new(&mut _countries, Nargs::AtLeastOne),
            "country",
            None,
        ))
        .add(
            Parameter::argument(Collection::new(&mut items, Nargs::AtLeastOne), "item")
                .help("The items."),
        )
        .build();
    parser.parse();
    println!("Items: {items:?}");
    execute(_verbose, operand, initial, _countries, items);
}

fn execute(
    _verbose: bool,
    operand: Operand,
    initial: Option<u32>,
    _countries: HashSet<Country>,
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
