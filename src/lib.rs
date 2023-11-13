//! `blarg` is a command line parser for Rust.
//!
//! Although other crates provide command line parser functionality, we have found they prioritize different concerns than those we are interested in.
//! It is very possible those crates can be configured to make *our desired* command line parser.
//! We built `blarg` to create our desired style of command line parser "out of the box".
//! Specifically, `blarg` attempts to prioritize the following design concerns:
//! * *Type safe argument parsing*:
//! The user should not call any `&str -> T` conversion functions directly.
//! * *Domain sensitive argument parsing*:
//! The user should not validate/reject any domain invalid inputs (see footnotes #1 for examples).
//! Instead, the command line parser should be configurable to prevent these.
//! * *Argument vs. option paradigm*:
//! The basic Api design for constructing a command line parser is via *arguments* and *options*.
//! Briefly, arguments are required parameters specified positionally on the Cli.
//! Options are optional parameters specified via `--..` or `-..` syntax.
//! * *Sub-command paradigm*:
//! The user may configure sub-commands which act to collect multiple related programs into a single Cli.
//! * *Detailed yet basic UX*:
//! The help and error output of the Cli should be very detailed, leaving no ambiguity in how to use the program.
//! However, we do not aim to support rich display configurations, such as colour output, shell completions, etc.
//! * *Reasonable performance*:
//! The command line parser should be *fast enough*.
//! To be clear, we are of the opinion that the cost of argument parsing is insignificant with respect to any non-trivial program.
//! That said, `blarg` will still aim to minimize its memory & CPU footprint, within reason.
//!
//! As it currently stands, we feel `blarg` fits a niche role in the rust ecosystem.
//! We hope you do as well!
//!
//! # Usage
//! This page includes a few demos on using `blarg.`
//! More examples are outlined in [the source](https://github.com/sawatzkylindsey/blarg/tree/main/examples).
//!
//! via [derive Api](./derive/index.html):
//! ```no_run
#![doc = include_str!("../examples/demo_summer_d.rs")]
//! ```
//! or equivalently via builder Api (this page):
//! ```no_run
#![doc = include_str!("../examples/demo_summer_b.rs")]
//! ```
//!
//! Both of these generate the same Cli program (with minor help message differences):
//! ```console
//! $ summer -h
//! usage: summer [-h] ITEM [...]
//! positional arguments:
//!  ITEM [...]  The items to sum.
//! options:
//!  -h, --help  Show this help message and
//!              exit.
//!
//! $ summer 1 2 3
//! Sum: 6
//!
//! $ summer
//! Parse error: Not enough tokens provided to parameter 'ITEM'.
//!
//! ^
//!
//! $ summer 1 blah
//! Parse error: 'blah' cannot convert to u32.
//! 1 blah
//!   ^
//! ```
//!
//! # Derive Api
//! We highly recommend using the [derive Api](./derive/index.html) to configure your Cli program.
//! The next section explains the structure and semantics of `blarg` using the builder Api, which applies to both builder and derive Apis.
//!
//! # Builder Api
//! Configure `blarg` by starting with a [`CommandLineParser`] and `add`ing parameters.
//! There are two classes of parameters: [`Parameter::argument`] and [`Parameter::option`].
//!
//! Each parameter takes a *field* which serves to specify the following aspects on the Cli:
//! * The underlying type `T` of the parameter (ex: `u32`).
//! * Whether `T` is wrapped in a container type `C` (ex: `Vec<T>` or `Option<T>`).
//! * The cardinality of the parameter (ex: 0, 1, N, at least 1, etc).
//!
//! All type `T` parsing in `blarg` is controlled by [`std::str::FromStr`].
//! `blarg` will parse any parameter type `T`, as long as it implements `std::str::FromStr`.
//!
//! The other aspects of parameter configuration relate to additional Cli usage and optics:
//! * Parameter naming, including the short name of an `Parameter::option`.
//! * Description of the parameter when displaying `--help`.
//!
//! ### Fields
//! * [`Scalar`]: defines a single-value `Parameter` (applies to both `Parameter::argument` & `Parameter::option`).
//! This is the most common field to use in your Cli.
//! * [`Collection`]: defines a multi-value `Parameter` (applies to both `Parameter::argument` & `Parameter::option`).
//! This field allows you to configure the cardinality (aka: `Nargs`) for any collection that implements [Collectable](./prelude/trait.Collectable.html).
//! `blarg` provides this `Collectable` implementations for `Vec<T>` and `HashSet<T>`.
//! * [`Switch`]: defines a no-value `Parameter::option` (not applicable to `Parameter::argument`).
//! This is used when specifying Cli *flags* (ex: `--verbose`).
//! Note that `Switch` may apply to any type `T` (not restricted to just `bool`).
//! * [`Optional`]: defines a `Parameter::option` (not applicable to `Parameter::argument`).
//! This field is used exclusively to specify an `Option<T>` type.
//!
//! ### Sub-commands
//! To setup a sub-command based Cli, start with a root `CommandLineParser`.
//! Both options and arguments may be added to the root parser via `add`.
//! The sub-command section of the parser begins by `branch`ing this parser.
//!
//! Branching takes a special [`Condition`] parameter which only allows a `Scalar` field.
//! You may describe the sub-commands on the condition via [Condition::choice](./struct.Condition.html#method.choice).
//! In `blarg`, any type `T` can be used to define sub-commands; sub-commands needn't only be strings.
//! See the **Condition** section below for further explanation.
//!
//! Once `branch`ed, the result is a [`SubCommandParser`] that allows you to setup individual sub-commands.
//! These are configured via [`SubCommandParser::command`], which takes the variant of `T` to which the sub-command applies, and a `impl FnOnce(SubCommand) -> SubCommand` to setup the parser.
//! From here, setup the sub-command via [`SubCommand::add`].
//!
//! Notice, the sub-command structure is dictated solely by the usage of `command`; usage of `choice` affects the display documentation only.
//! As a side effect of this distinction, you may include "undocumented" sub-commands (as well as "false" sub-commands), both shown in the example below.
//!
//! ```no_run
#![doc = include_str!("../examples/demo_sub_command.rs")]
//! ```
//!
//! ```console
//! usage: sub-command [-h] SUB
//! positional arguments:
//!  SUB         {1, 2, 3}
//!    1           the one sub-command
//!    2           the two sub-command
//!    3           the three sub-command
//! <truncated>
//!
//! $ sub-command 0 -h
//! usage: sub-command 0 [-h] ARG
//! positional arguments:
//!  ARG
//! options:
//!  -h, --help  Show this help message and
//!              exit.
//!  --opt
//!
//! $ sub-command 0 true
//! Used sub-command '0'.
//! arg_0: true
//! opt_0: false
//!
//! $ sub-command 0 false --opt
//! Used sub-command '0'.
//! arg_0: false
//! opt_0: true
//!
//! $ sub-command 2
//! Used sub-command '2'.
//! argument-less & option-less
//!
//! $ sub-command 3
//! Parse error: Unknown sub-command '3' for parameter 'SUB'.
//! 3
//! ^
//! ```
//!
//! **Condition**</br>
//! In order to support arbitrary branching types `T`, we use an implicit (not compile-time enforced) requirement.
//! Simply, [`std::str::FromStr`] for `T` must be inverted by [`std::fmt::Display`] for the same type `T`.
//! In code, this means the following assertion must succeed.
//!
//! ```ignore
//! let s: &str = "..";
//! let s_prime: String = T::from_str(s).unwrap().to_string();
//! assert_eq!(s_prime, s);
//! ```
//!
//! For more details on this requirement, see the [`Condition`] documentation.
//!
//! ### Defaults & Initials
//! Technically, `blarg` has nothing to do with specifying default values for parameters.
//! This may be confusing - defaults are a common feature for command line parsers!
//! Instead, the defaults of your Cli will come from the variable initializations when configuring `blarg`.
//! We support presenting *initials* over the help message via [derive Api](./derive/index.html), but behaviourally `blarg` does not take part in *setting* parameter defaults.
//!
//! ```
//! // The default for the 'verbose' parameter is 'false'.
//! let mut verbose: bool = false;
//! // The default for the 'value' parameter is '0'.
//! let mut value: u32 = 0;
//!
//! // Use `verbose` and `value` in the CommandLineParser.
//! // `GeneralParser::parse` will assign onto these variables.
//! ```
//!
//! We'd also like to point out: semantically, defaults only apply to options (`Parameter::option`).
//! By definition, arguments (`Parameter::argument`) must be specified on the Cli, so having a "default" does not make sense.
//!
//! In the case of `Collection` parameters (for both options and arguments), the *initial* value again comes from the variable initialization.
//! As parameters are received from the Cli input, these are *added to* the collection (via `Collectable`).
//! This may be unexpected if you think of setting a default value for the `Collection`, which is then reset upon receiving input.
//!
//! When using `blarg`, we recommend thinking in terms of *initial* values that are later affected by the Cli invocation.
//! In the case of non-`Collection` parameters, the initial value will be overwritten, but in the case of `Collection` parameters, the initial value will be extended.
//!
//! ```
//! // The initial for the 'items' parameter is '[0, 1, 2]'.
//! let mut items: Vec<u32> = vec![0, 1, 2];
//!
//! // Use `items` in the CommandLineParser.
//! // `GeneralParser::parse` will `Collectable::add` to `items`.
//! ```
//!
//! ### Organization
//! It may be useful to organize your program variables into a single struct.
//! Configuring such an organizational struct is made seamless with the [derive Api](./derive/index.html).
//! The following demonstrates how to *manually* configure an organizational struct with `blarg`.
//!
//! ```no_run
#![doc = include_str!("../examples/demo_organization.rs")]
//! ```
//!
//! # Cli Semantics
//! `blarg` parses the Cli tokens according to the following set of rules.
//! By and large this syntax should be familiar to many Cli developers, with a few subtle nuances for various edge cases.
//!
//! * Each parameter matches a number of tokens based off its cardinality.
//! * Arguments are matched based off positional ordering.
//! Once the expected cardinality is matched, then the parser naturally switches to the next parameter.
//! For example, `a b c` will match `a b` into a cardinality=2 argument, and `c` into the next argument.
//! * Options are matched based off the `--NAME` (or short name `-N`) specifier.
//! Once specified, the cardinality is matched against the subsequent tokens.
//! For example, `--key x y` will match `x` and `y` into a cardinality=2 option.
//! Again, when the expected cardinality is matched, then the parser switches to the next parameter.
//! * In both arguments and options, the `Nargs` `*` and `+` match greedily; they never switch over to the next parameter.
//! This greedy matching can be broken by using an option as a separator (see footnotes #2 for guidance).
//! For example, `a b c --key value d e f` will match `a b c` into the first greedy argument, and `d e f` into the second (assuming `--key` is a cardinality=1 option).
//! * The key-value pair of a cardinality=1 option may be separated with the `=` character.
//! Subsequent tokens always rollover to the next parameter, even if the option's cardinality is greedy.
//! For example, `--key=123` is equivalent to `--key 123`.
//! Also notice, only the first `=` character is used as a separator.
//! For example, `--key=123=456` is equivalent to `--key 123=456` (see footnotes #3 for guidance).
//! * The previous rule also applies to cardinality=1 options using the short name syntax.
//! For example, `-k=123` is equivalent to `--key 123`.
//! * Multiple short named options may be combined into a single flag.
//! For example, `-abc` is equivalent to `--apple --banana --carrot`.
//! The `=` separator rule may be applied *only* to the final option in this syntax.
//! For example, `-abc=123` is equivalent to `--apple --banana --carrot=123`.
//!
//!
//! ### Field-Narg Interaction
//! **Argument**</br>
//! ```console
//! Parameter         | Narg | Cardinality | Syntax           | Description
//! -----------------------------------------------------------------------------------------------
//! Scalar<T>         |      | [1]         | VALUE            | precisely 1
//! Collection<C<T>>  | n    | [n]         | VALUE .. VALUE   | precisely n
//! Collection<C<T>>  | *    | [0, ∞)      | [VALUE ...]      | any amount; captured greedily
//! Collection<C<T>>  | +    | [1, ∞)      | VALUE [...]      | at least 1; captured greedily
//! ```
//!
//! **Option**</br>
//! ```console
//! Parameter         | Narg | Cardinality | Syntax                   | Description
//! -------------------------------------------------------------------------------------------------
//! Scalar<T>         |      | [1]         | [--NAME VALUE]           | precisely 1
//! Collection<C<T>>  | n    | [n]         | [--NAME VALUE .. VALUE]  | precisely n
//! Collection<C<T>>  | *    | [0, ∞)      | [--NAME [VALUE ...]]     | any amount; captured greedily
//! Collection<C<T>>  | +    | [1, ∞)      | [--NAME VALUE [...]]     | at least 1; captured greedily
//! Switch<T>         |      | [0]         | [--NAME]                 | precisely 0
//! Optional<T>       |      | [1]         | [--NAME VALUE]           | precisely 1
//! ```
//!
//! # Footnotes
//! 1. Examples of domain sensitive argument parsing:
//!     * A collection that accepts a precise number of values: `triple-input-program 1 2 3`
//!     * A collection that de-duplicates values: `set-input-program 1 2 1`
//! 2. Although the greedy matching can be broken by an option, `blarg` does not recommend a Cli design that requires this tactic.
//! Clis that use more than one `*` or `+` greedy parameter are complicated, and put a significant burden on the user to understand how to break the greedy matching.
//! 3. Using the equals sign inside a parameter can be a useful way to parse complex structs.
//! In other words, you can write a custom `std::str::FromStr` deserializer.
//! For example, `a=123,b=456` could be deserialized into `struct MyStruct { a: u32, b: u32 }`.
//!
//! # Features
//! * `unit_test`: For features that help with unit testing.
pub mod derive;
pub use blarg_builder::*;
