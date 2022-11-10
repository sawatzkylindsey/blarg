# blarg
`blarg` is an opinionated command line argument parser for Rust.
Although there are other suitable argument parser crates, in my experience they center around CLI/UX paradigms that differ from what I desire in an argument parser.
If you are happy with the other argument parser libraries in Rust, I would highly recommend using them instead of this one.
Otherwise, read on.

"blarg" is derived from the following:
* **arg** for argument parser.
* **bl**arg because all the other names are already taken.

### CLI Paradigm
The CLI design this parser aims to achieve follows a unix-like standard (if one were to exist).
Of course, CLI design has changed much over the years - I won't attempt to point to a specific period and say *this* is what `blarg` provides.
Nevertheless, let's flesh this out a little.

Our target CLI design is inspired primary by Python's [ArgumentParser](https://docs.python.org/3/library/argparse.html).
To be clear, the functionaly and UX provided by `ArgumentParser` can be used as a rough guideline for the functionality and UX provided by `blarg`.
However, the programmatic interface may vary significantly.
[This guide](https://clig.dev) provides another reasonable sketch for our target CLI design, but to be clear does not wholly align with `blarg`'s position.
Aside from what are probably straightforward standards (provide help via `-h`, `--help`, return `0` on success and `non-zero` on error, etc), these are `blarg`'s primary concerns:

* Arguments are *required* parameters to the CLI.
They are always positional.
Their order is important to the program semantics.
They are never specified via some key.
Example of an argument: `'mkdir NAME'`
* Options are the *optional* parameters to the CLI.
They are specified via "single-dash single-char" or "double-dash word" keys, and may accept values or indicate flags (the implicit boolean value).
Their order is not important to the program semantics.
Example of an option: `'mkdir [-p] ..'` or `'mkdir [-m MODE] ..'` or `'diff [-i | --ignore-case] ..'`
* To re-iterate, the required parameters to a CLI are always described via positional arguments.
The optional parameters to the CLI are always described via key-value pair.
* Sometimes, arguments or options take some number of parameters.
The CLI should present this semantic clearly, and produce an error when encountering an invalid number.
* When the CLI arguments and/or options becomes too confusing to effectivley use, this is an indication that the CLI needs re-design.
There are two common re-design patterns:
    1. Use configuration file(s).
    2. Break the CLI into smaller point-focused CLIs.
       This can be done via use of separate CLI binaries, or via sub-command structure (ex: `'git add ..'` and `'git commit ..'`).

### Blarg Api
The `blarg` Api is a work in progress.
In principle, we provide a reasonably idiomatic and type-safe interface for the above CLI paradigm.
It should be straightforward to program arguments, options and sub-commands with the mentioned semantics.
On the other hand, semantics outside the CLI paradigm should be difficult/impossible within the `blarg` Api.
For example, the `blarg` Api does not allow for a required parameter to be specified via option syntax.

### Api Grammar

**Argument**
```
Parameter         | Narg | Grammar         | Description
---------------------------------------------------------------------------
Scalar<T>         | \    | VALUE           | precisely 1
Optional<T>       | \    | \               | invalid; use Parameter::option
Collection<C<T>>  | n    | VALUE .. VALUE  | precisely n
Collection<C<T>>  | *    | [VALUE ...]     | any amount; captured greedily
Collection<C<T>>  | +    | VALUE [...]     | at least 1; captured greedily
Switch<T>         | \    | \               | invalid; use Parameter::option
```

1. The CLI inputs are matched to arguments based off positional ordering.
Once the precise number of inputs are matched, then the input naturally switches to the next argument.
For example, `1 2 3` will match `1 2` into an n=2 argument, and `3` into the next argument.
2. The nargs `*` and `+` match greedily; they never switch over to the next argument.
This greedy matching can be broken by using an option as a separator (that said, `blarg` does not recommend a CLI design more than one `*` or `+` argument).
For example, `1 2 3 --key 123 4 5 6` will match `1 2 3` into the first `*` or `+` argument, and `4 5 6` into the second.


**Option**
```
Parameter         | Narg | Grammar                  | Description
-----------------------------------------------------------------------------------
Scalar<T>         | \    | [--NAME VALUE]           | precisely 1
Optional<T>       | \    | [--NAME VALUE]           | precisely 1
Collection<C<T>>  | n    | [--NAME VALUE .. VALUE]  | precisely n
Collection<C<T>>  | *    | [--NAME VALUE ...]       | any amount; captured greedily
Collection<C<T>>  | +    | [--NAME VALUE [...]]     | at least 1; captured greedily
Switch<T>         | \    | [--NAME]                 | precisely 0
```
There are two addition rules for the grammar of **options**.

1. The key-value pair may be separated with the `=` character.
In this case, only 1 value is passed to the option; subsequent tokens always apply to the next parameter.
In other words, this overrides a greedy capture.
For example, `--key=123` is equivalent to `--key 123`.
Only the first `=` character is used as a separator.
For example, `--key=123=456` is equivalent to `--key 123=456`.
2. If there is a short name, then the same grammar may be used with a single-dash single-char flag.
Multiple short named options may be combined into a single flag.
For example, `-abc` is equivalent to `--apple --banana --carrot`.
The previous rule applies to short names as well.
For example, `-abc=123` is equivalent to `--apple --banana --carrot=123`.


### Development

    cargo build
    cargo test

    ./target/debug/examples/reducer -h
