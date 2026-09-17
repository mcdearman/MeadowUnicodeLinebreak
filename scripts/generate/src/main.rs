//! Writes `src/Tables.mw` and `src/Cases.mw` from `unicode-linebreak`.
//!
//! ```text
//! cargo run --release -- <package root>
//! ```
//!
//! * **Tables** -- the line break class of every code point, the crate's pair
//!   table (its whole state machine, as data), and which pairs of classes are
//!   safe places to start breaking from.
//! * **Cases** -- strings with the break opportunities the crate finds in
//!   them, and where it splits them for `split_at_safe`.
//!
//! What is left, `src/Break.mw`, is the few lines of the crate's `lib.rs` that
//! drive the table. They are fingerprinted, so that a new version of the crate
//! that changes them is not picked up unnoticed.

#[allow(dead_code, clippy::all, non_upper_case_globals)]
mod upstream {
    include!(concat!(env!("OUT_DIR"), "/upstream.rs"));
}

use std::fmt::Write as _;
use std::path::PathBuf;
use unicode_linebreak::{BreakOpportunity, break_property, linebreaks, split_at_safe};
use upstream::BreakClass;

const SCALARS: u32 = 0x11_0000;

/// The crate version pinned in `Cargo.toml`.
const UPSTREAM_VERSION: &str = "0.1.5";

/// The fingerprint of the crate's `lib.rs`, which `src/Break.mw` ports.
const DRIVER: u64 = 0xfc66_1bcc_0def_2334;

/// How many classes there are, plus one for end of text: the pair table's
/// width.
const COLUMNS: usize = 44;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "../..".into()));

    let print = fingerprint(include_str!(concat!(env!("OUT_DIR"), "/lib.rs.txt")));
    if print != DRIVER {
        eprintln!(
            "error: the unicode-linebreak driver is not the one src/Break.mw ports.\n\
             Compare src/lib.rs in {} with the previous version, carry any change\n\
             into src/Break.mw, then set DRIVER in scripts/generate/src/main.rs to\n\
             {print:#x}",
            env!("UPSTREAM_DIR")
        );
        std::process::exit(1);
    }

    let tables = tables();
    let tables_path = root.join("src/Tables.mw");
    std::fs::write(&tables_path, &tables).unwrap();
    eprintln!("wrote {} ({} bytes)", tables_path.display(), tables.len());

    let cases = cases();
    let cases_path = root.join("src/Cases.mw");
    std::fs::write(&cases_path, &cases).unwrap();
    eprintln!("wrote {} ({} bytes)", cases_path.display(), cases.len());
}

/// FNV-1a: stable across builds, which `DefaultHasher` does not promise.
fn fingerprint(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

// --- encoding -----------------------------------------------------------------------

/// A number as `digits` base-64 digits, most significant first, each digit the
/// character `'0' + d`: `'0'` to `'o'`, one contiguous run of ASCII.
fn digits(out: &mut String, value: u64, digits: u32) {
    assert!(
        value < 1 << (6 * digits),
        "{value} does not fit in {digits} digits"
    );
    for k in (0..digits).rev() {
        out.push(char::from(b'0' + ((value >> (6 * k)) & 63) as u8));
    }
}

/// `text` as one Meadow string literal, broken with `\`-newline every `width`
/// characters. Only printable ASCII is written raw; a space that would start a
/// line is `\x20`, since a continuation drops leading whitespace.
fn long_literal(text: &str, width: usize) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / width * 4 + 2);
    out.push('"');
    for (i, c) in text.chars().enumerate() {
        let line_start = i > 0 && i % width == 0;
        if line_start {
            out.push_str("\\\n    ");
        }
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            ' ' if line_start => out.push_str("\\x20"),
            ' '..='~' => out.push(c),
            _ => {
                let _ = write!(out, "\\u{{{:X}}}", u32::from(c));
            }
        }
    }
    out.push('"');
    out
}

const HEADER: &str = "\
-- Copyright Axel Forsman, and the Meadow port's authors. Licensed under the
-- Apache License, Version 2.0: see LICENSE and COPYRIGHT.";

// --- tables -------------------------------------------------------------------------

/// Every class, in declaration order, which is what `as u8` numbers them by.
const ALL: [BreakClass; COLUMNS - 1] = {
    use BreakClass::*;
    [
        Mandatory,
        CarriageReturn,
        LineFeed,
        CombiningMark,
        NextLine,
        Surrogate,
        WordJoiner,
        ZeroWidthSpace,
        NonBreakingGlue,
        Space,
        ZeroWidthJoiner,
        BeforeAndAfter,
        After,
        Before,
        Hyphen,
        Contingent,
        ClosePunctuation,
        CloseParenthesis,
        Exclamation,
        Inseparable,
        NonStarter,
        OpenPunctuation,
        Quotation,
        InfixSeparator,
        Numeric,
        Postfix,
        Prefix,
        Symbol,
        Ambiguous,
        Alphabetic,
        ConditionalJapaneseStarter,
        EmojiBase,
        EmojiModifier,
        HangulLvSyllable,
        HangulLvtSyllable,
        HebrewLetter,
        Ideographic,
        HangulLJamo,
        HangulVJamo,
        HangulTJamo,
        RegionalIndicator,
        ComplexContext,
        Unknown,
    ]
};

/// The class the crate gives every code point, surrogates included: it takes a
/// `u32`, not a `char`.
fn class_runs() -> Vec<(u32, u32, u8)> {
    let mut out: Vec<(u32, u32, u8)> = Vec::new();
    for cp in 0..SCALARS {
        let v = break_property(cp) as u8;
        if v == unicode_linebreak::BreakClass::Unknown as u8 {
            continue;
        }
        match out.last_mut() {
            Some((_, hi, lv)) if *lv == v && *hi + 1 == cp => *hi = cp,
            _ => out.push((cp, cp, v)),
        }
    }
    out
}

fn tables() -> String {
    for (i, c) in ALL.iter().enumerate() {
        assert_eq!(*c as usize, i, "{c:?} is not numbered {i}");
    }
    assert_eq!(usize::from(upstream::eot), COLUMNS - 1);
    assert_eq!(upstream::PAIR_TABLE[0].len(), COLUMNS);
    let (maj, min, pat) = unicode_linebreak::UNICODE_VERSION;

    let mut classes = String::new();
    for (lo, hi, v) in class_runs() {
        digits(&mut classes, lo.into(), 4);
        digits(&mut classes, hi.into(), 4);
        digits(&mut classes, v.into(), 1);
    }

    let mut pairs = String::new();
    for row in upstream::PAIR_TABLE.iter() {
        for &v in row {
            digits(&mut pairs, v.into(), 2);
        }
    }

    let mut safe = String::new();
    for a in ALL {
        for b in ALL {
            safe.push(if upstream::is_safe_pair(a, b) {
                '1'
            } else {
                '0'
            });
        }
    }

    let listing = ALL
        .iter()
        .enumerate()
        .map(|(i, c)| format!("--   {i:>2} {c:?}"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = String::new();
    let _ = writeln!(
        out,
        "-- GENERATED by scripts/generate.sh from unicode-linebreak {UPSTREAM_VERSION}.
-- Do not edit: run the script again instead.
--
-- Every table is a string of fixed-width records, each field written in the
-- base-64 digits '0' ('0' + 0) to 'o' ('0' + 63), most significant first, and
-- is read in place by `Lookup.mw`.
--
{HEADER}

-- The Unicode version the tables describe.
@pub(pkg) def unicodeVersion = ({maj}, {min}, {pat})

-- The line break class of each code point, as `lo hi` (4 digits each) and the
-- class (1). Code points not listed are Unknown. The classes:
--
{listing}
@pub(pkg) def classes =
  {}

-- The state machine: {} rows, one per state, of {COLUMNS} entries, one per class
-- and then end of text ({}), 2 digits each. An entry is the next state, plus
-- 128 if there is a break before the class and 64 if it is mandatory. The
-- start state is {}.
@pub(pkg) def pairs =
  {}

-- Whether a pair of classes (row, then column; {} of each) is a safe place to
-- resume breaking from: '1' or '0'.
@pub(pkg) def safePairs =
  {}",
        long_literal(&classes, 96),
        upstream::PAIR_TABLE.len(),
        upstream::eot,
        upstream::sot,
        long_literal(&pairs, 96),
        ALL.len(),
        long_literal(&safe, 86),
    );
    out
}

// --- cases --------------------------------------------------------------------------

/// A small deterministic generator, so that the cases are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Two characters of every class, and some everyday ones.
fn interesting() -> Vec<char> {
    let mut seen: std::collections::BTreeMap<u8, Vec<char>> = Default::default();
    for cp in 0..SCALARS {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        let list = seen.entry(break_property(cp) as u8).or_default();
        if list.len() < 2 {
            list.push(c);
        }
    }
    let mut picks: Vec<char> = seen.into_values().flatten().collect();
    picks.extend(" -\u{ad}.,!?()'\"0123456789aZ\n\r\t\u{200d}\u{a0}".chars());
    picks.sort_unstable();
    picks.dedup();
    picks
}

fn cases() -> String {
    let mut strings: Vec<String> = [
        "",
        "Hello world!",
        "a b \nc",
        "Not allowed to break within em dashes: — —",
        "The quick (\"brown\") fox can't jump 32.3 feet, right?",
        "e.g. $10,000.00 — 50% off! (see: http://example.com/a-b?c=d)",
        "日本語のテキスト、カタカナ。「引用」です。",
        "👨\u{200d}👩\u{200d}👧 👍🏽 🇺🇸🇬🇧",
        "co-operate re-\u{ad}enter x\u{2014}y 1\u{2013}2",
        "\r\n\r\n\u{85}\u{2028}\u{2029}\u{b}\u{c}",
    ]
    .map(String::from)
    .to_vec();

    let pool = interesting();
    let mut rng = Rng(0x5eed_11e0_b4ea_c0de);
    for _ in 0..5000 {
        let len = 1 + rng.below(10);
        strings.push((0..len).map(|_| pool[rng.below(pool.len())]).collect());
    }
    strings.sort();
    strings.dedup();

    let mut body = String::new();
    for s in &strings {
        digits(&mut body, s.len() as u64, 3);
        body.push_str(s);
        let breaks: Vec<(usize, BreakOpportunity)> = linebreaks(s).collect();
        digits(&mut body, breaks.len() as u64, 3);
        for (at, kind) in breaks {
            let mandatory = u64::from(kind == BreakOpportunity::Mandatory);
            digits(&mut body, 2 * at as u64 + mandatory, 3);
        }
        digits(&mut body, split_at_safe(s).0.len() as u64, 3);
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "-- GENERATED by scripts/generate.sh from unicode-linebreak {UPSTREAM_VERSION}.
-- Do not edit: run the script again instead.
--
-- Strings with the break opportunities the crate finds in them, for
-- `Tests.mw`: {} in all.
--
{HEADER}

-- Each: the length in bytes (3 digits) and that many bytes; how many breaks
-- (3), then each as twice its byte offset, plus one if it is mandatory (3);
-- then where `split_at_safe` splits the string (3).
@cfg(test)
@pub(pkg) def cases =
  {}",
        strings.len(),
        long_literal(&body, 96)
    );
    out
}
