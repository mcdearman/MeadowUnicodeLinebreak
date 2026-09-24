# unicodeLinebreak

Find where a line of text may be broken, for
[Meadow](https://github.com/mcdearman/meadow), using the Line Breaking
Algorithm of [UAX #14](https://www.unicode.org/reports/tr14/).

This package is a port of Rust's
[`unicode-linebreak`](https://github.com/axelf4/unicode-linebreak) 0.1.5 by
Axel Forsman, and covers Unicode 15.0.0. [textwrap](https://github.com/mcdearman/Baler)
uses it to find where it may break lines.

## Install

```sh
meadow add mcdearman/UnicodeLinebreak
```

## Use

```meadow
use UnicodeLinebreak (linebreaks, breakProperty, BreakClass)

def main =
  ( linebreaks "a b \nc",
    -- [(2, Allowed), (5, Mandatory), (6, Mandatory)]
    breakProperty 0x2CF3 == BreakClass.Alphabetic
  )
```

| | |
|---|---|
| `linebreaks s` | every break opportunity in `s`, as `(offset, Allowed or Mandatory)`. The offset is the byte where the next line would start. The end of a non-empty string always counts as a mandatory break. |
| `breakProperty cp` | the line break class (`BreakClass`) of the code point `cp` |
| `splitAtSafe s` | splits `s` into `(before, after)` at the last point where breaking can restart without looking back. This saves re-scanning the whole string when a long line needs its breaks recomputed near the end. |
| `unicodeVersion` | `(15, 0, 0)` |

`BreakOpportunity`'s constructors, `Allowed` and `Mandatory`, are exported
bare. `BreakClass`'s constructors are written `BreakClass.Alphabetic` and so
on, unless you `use UnicodeLinebreak.BreakClass.*`.

As in the crate, Complex-Context Dependent characters (Thai, Lao, Khmer, …)
are treated as ordinary letters. Finding breaks inside words in those scripts
needs a dictionary, and this package has none.

## How it's made

- **`src/Tables.mw`** is generated from the crate. It holds the class of every
  code point and the crate's pair table, which encodes the whole algorithm as
  a state machine.
- **`src/Break.mw`** translates the few lines of the crate's `lib.rs` that run
  that state machine.
- **`src/Cases.mw`** is generated test data: 4,606 strings built from
  characters of every class, plus some real text. The expected break
  opportunities and `splitAtSafe` results come from calling the crate itself,
  and `meadow test` checks that this port matches.

To regenerate, update the version pin in `scripts/generate/Cargo.toml` and run
`scripts/generate.sh`. It needs a Rust toolchain. If the crate's `lib.rs` has
changed, the generator stops so that `src/Break.mw` can be checked against it
first.

## Licence

Licensed under the [Apache License, Version 2.0](LICENSE), like the crate. See
[COPYRIGHT](COPYRIGHT).
