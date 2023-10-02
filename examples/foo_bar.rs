use blarg::{
    prelude::*, Collection, CommandLineParser, Condition, Nargs, Optional, Parameter, Scalar,
    Switch,
};
use std::collections::HashSet;
use std::hash::Hash;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash)]
enum FooBar {
    Foo,
    Bar,
}

impl std::fmt::Display for FooBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FooBar::Foo => write!(f, "foo"),
            FooBar::Bar => write!(f, "bar"),
        }
    }
}

impl FromStr for FooBar {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "foo" => Ok(FooBar::Foo),
            "bar" => Ok(FooBar::Bar),
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
    let mut foo_bar = FooBar::Foo;
    let mut initial: Option<u32> = None;
    let mut countries: HashSet<Country> = HashSet::default();
    let mut items: Vec<u32> = Vec::default();

    let ap = CommandLineParser::new("foo_bar");
    let parser = ap
        .add(
            Parameter::option(Switch::new(&mut verbose, true), "verbose", Some('v'))
                .help("Do dee doo."),
        )
        .branch(
            Condition::new(Scalar::new(&mut foo_bar), "foo_bar")
                .choice(FooBar::Foo, "abc 123")
                .choice(FooBar::Bar, "abc 123")
                .help("foo'y bar'y stuff"),
        )
        .command(FooBar::Foo, |sub| {
            sub.add(Parameter::option(
                Optional::new(&mut initial),
                "initial",
                None,
            ))
            .add(
                Parameter::argument(Collection::new(&mut items, Nargs::Any), "item")
                    .help("The items."),
            )
        })
        .command(FooBar::Bar, |sub| {
            sub.add(Parameter::option(
                Collection::new(&mut countries, Nargs::AtLeastOne),
                "country",
                None,
            ))
        })
        .build()
        .expect("Invalid argument parser configuration");
    parser.parse();
    println!("Items: {items:?}");
    execute(verbose, foo_bar, initial, countries, items);
}

fn execute(
    _verbose: bool,
    foo_bar: FooBar,
    initial: Option<u32>,
    countries: HashSet<Country>,
    items: Vec<u32>,
) {
    match foo_bar {
        FooBar::Foo => {
            println!("Foo: {initial:?} {items:?}");
        }
        FooBar::Bar => {
            println!("Bar: {countries:?}");
        }
    };
}
