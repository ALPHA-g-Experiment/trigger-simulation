use crate::gen::{Positive, WireEvent, WirePattern};
use std::fmt;
use std::ops::Add;
use winnow::ascii::{hex_uint, newline};
use winnow::combinator::{delimited, opt, separated, terminated};
use winnow::error::ContextError;
use winnow::Parser;

const TABLE_SIZE: usize = 2usize.pow(16);

/// Set of [`WirePattern`]s.
///
/// The [`LookupTable`] determines the set of wire patterns of interest that
/// produce a TRG signal out of the MLU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LookupTable {
    inner: [bool; TABLE_SIZE],
}

impl LookupTable {
    /// Creates a new empty lookup table.
    ///
    /// # Examples
    ///
    /// ```
    /// use trg::mlu::LookupTable;
    /// let table = LookupTable::new();
    /// ```
    pub fn new() -> Self {
        Self {
            inner: [false; TABLE_SIZE],
        }
    }
    /// Adds a wire pattern to the lookup table. Returns whether the pattern was
    /// newly inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// # use trg::gen::WirePattern;
    /// use trg::mlu::LookupTable;
    ///
    /// let mut table = LookupTable::new();
    ///
    /// assert_eq!(table.insert(WirePattern::from_bits(0)), true);
    /// assert_eq!(table.insert(WirePattern::from_bits(0)), false);
    /// ```
    pub fn insert(&mut self, wire_pattern: WirePattern) -> bool {
        let index = usize::from(wire_pattern.0);
        let was_inserted = !self.inner[index];
        self.inner[index] = true;

        was_inserted
    }
    /// Returns `true` if the given wire pattern is in the lookup table.
    ///
    /// # Examples
    ///
    /// ```
    /// # use trg::gen::WirePattern;
    /// use trg::mlu::LookupTable;
    ///
    /// let table = LookupTable::from([WirePattern::from_bits(0)]);
    /// assert_eq!(table.contains(WirePattern::from_bits(0)), true);
    /// assert_eq!(table.contains(WirePattern::from_bits(1)), false);
    /// ```
    pub fn contains(&self, wire_pattern: WirePattern) -> bool {
        let index = usize::from(wire_pattern.0);
        self.inner[index]
    }
    /// Removes a wire pattern from the lookup table. Returns whether the
    /// pattern was present in the table.
    ///
    /// # Examples
    ///
    /// ```
    /// # use trg::gen::WirePattern;
    /// use trg::mlu::LookupTable;
    ///
    /// let mut table = LookupTable::new();
    ///
    /// table.insert(WirePattern::from_bits(0));
    /// assert_eq!(table.remove(WirePattern::from_bits(0)), true);
    /// assert_eq!(table.remove(WirePattern::from_bits(0)), false);
    /// ```
    pub fn remove(&mut self, wire_pattern: WirePattern) -> bool {
        let index = usize::from(wire_pattern.0);
        let was_present = self.inner[index];
        self.inner[index] = false;

        was_present
    }
}

impl Default for LookupTable {
    /// Creates a new empty lookup table.
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<WirePattern> for LookupTable {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = WirePattern>,
    {
        let mut lookup_table = Self::new();
        for wire_pattern in iter {
            lookup_table.insert(wire_pattern);
        }

        lookup_table
    }
}

impl<const N: usize> From<[WirePattern; N]> for LookupTable {
    /// Converts a `[WirePattern; N]` into a `LookupTable`.
    ///
    /// If the array contains any equal values, all but one will be dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use trg::gen::WirePattern;
    /// use trg::mlu::LookupTable;
    ///
    /// let table1 = LookupTable::from([WirePattern::from_bits(0)]);
    /// let table2: LookupTable = [WirePattern::from_bits(0)].into();
    /// assert_eq!(table1, table2);
    /// ```
    fn from(arr: [WirePattern; N]) -> Self {
        Self::from_iter(arr)
    }
}

fn bit_pattern_string(n: u16) -> String {
    format!("{:016b}", n.reverse_bits())
        .replace("0", ".")
        .replace("1", "X")
}

fn bits_string(n: u16) -> String {
    format!("{} bits", n.count_ones())
}

fn clusters_string(n: u16) -> String {
    let mut count = 0;
    let mut in_cluster = n & (1 << 15) != 0;

    for i in 0..16 {
        if n & (1 << i) != 0 {
            if !in_cluster {
                count += 1;
                in_cluster = true;
            }
        } else {
            in_cluster = false;
        }
    }

    if count == 0 && in_cluster {
        count += 1;
    }

    format!("{count} clusters")
}

impl fmt::Display for LookupTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self
            .inner
            .iter()
            .enumerate()
            .filter(|(_, &is_present)| is_present)
            .map(|(n, _)| {
                format!(
                    "0x{n:04x} 1 {}, {}, {}",
                    bit_pattern_string(u16::try_from(n).unwrap()),
                    bits_string(u16::try_from(n).unwrap()),
                    clusters_string(u16::try_from(n).unwrap())
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        write!(f, "{text}")
    }
}

// There is no point in adding any `context` to these errors unless I change
// this to a "tokenize, then parse" approach (allowing for better semantic
// errors with spans). But given the use case, I don't think it's worth it. A
// simple "this line is wrong" is enough.
fn parse_line(input: &mut &str) -> winnow::Result<u16> {
    let n: u16 = delimited("0x", hex_uint, " 1 ").parse_next(input)?;

    let _ = (
        bit_pattern_string(n).as_str(),
        ", ",
        bits_string(n).as_str(),
        ", ",
        clusters_string(n).as_str(),
    )
        .parse_next(input)?;

    Ok(n)
}

/// The error type returned when parsing a [`LookupTable`] fails.
#[derive(Debug)]
pub struct ParseError {
    input: String,
    span: std::ops::Range<usize>,
}

impl ParseError {
    fn from_parse(error: winnow::error::ParseError<&str, ContextError>) -> Self {
        let input = error.input().to_string();
        let span = error.char_span();
        Self { input, span }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = annotate_snippets::Level::Error
            .title("invalid line starting here")
            .snippet(
                annotate_snippets::Snippet::source(&self.input)
                    .fold(true)
                    .annotation(annotate_snippets::Level::Error.span(self.span.clone())),
            );
        let renderer = annotate_snippets::Renderer::plain();
        let rendered = renderer.render(message);
        rendered.fmt(f)
    }
}

impl std::error::Error for ParseError {}

impl std::str::FromStr for LookupTable {
    type Err = ParseError;

    /// Parse a [`LookupTable`] from a string. The string should have the same
    /// format as processed by the real detector.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use trg::mlu::LookupTable;
    /// # use std::str::FromStr;
    /// let string = std::fs::read_to_string("mlu_file.txt")?;
    /// let table = LookupTable::from_str(&string)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut inner = [false; TABLE_SIZE];

        let () = terminated(
            separated(
                0..,
                parse_line.map(|n| {
                    let index = usize::from(n);
                    inner[index] = true;
                }),
                newline,
            ),
            opt(newline),
        )
        .parse(input)
        .map_err(ParseError::from_parse)?;

        Ok(Self { inner })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TrgSignal<T> {
    pub time: T,
}

#[derive(Clone, Copy, Debug)]
enum MluState<T> {
    Idle,
    // Accumulating wire patterns during the prompt window.
    Accumulate {
        // Time when accumulation will stop and a TRG decision will be made.
        stop_time: T,
        cumulative: WirePattern,
    },
    Wait {
        stop_time: T,
    },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Mlu<T> {
    state: MluState<T>,
    prompt_window: Positive<T>,
    wait_gate: Positive<T>,
    table: LookupTable,
}

impl<T> Mlu<T> {
    pub(super) fn new(
        prompt_window: Positive<T>,
        wait_gate: Positive<T>,
        table: LookupTable,
    ) -> Self {
        Self {
            state: MluState::Idle,
            prompt_window,
            wait_gate,
            table,
        }
    }
}

impl<T> Mlu<T>
where
    T: Add<Output = T> + PartialOrd + Clone,
{
    pub(super) fn process(&mut self, event: &WireEvent<T>) -> Option<TrgSignal<T>> {
        match std::mem::replace(&mut self.state, MluState::Idle) {
            MluState::Accumulate {
                stop_time,
                cumulative,
            } => {
                if event.time < stop_time {
                    self.state = MluState::Accumulate {
                        stop_time,
                        cumulative: cumulative | event.wire_pattern,
                    };
                    None
                } else if event.time < stop_time.clone() + self.wait_gate.inner().clone() {
                    self.state = MluState::Wait {
                        stop_time: event.time.clone() + self.wait_gate.inner().clone(),
                    };
                    match self.table.contains(cumulative) {
                        true => Some(TrgSignal { time: stop_time }),
                        false => None,
                    }
                } else {
                    self.state = MluState::Accumulate {
                        stop_time: event.time.clone() + self.prompt_window.inner().clone(),
                        cumulative: event.wire_pattern,
                    };
                    match self.table.contains(cumulative) {
                        true => Some(TrgSignal { time: stop_time }),
                        false => None,
                    }
                }
            }
            MluState::Wait { stop_time } => {
                if event.time < stop_time {
                    self.state = MluState::Wait {
                        stop_time: event.time.clone() + self.wait_gate.inner().clone(),
                    };
                    None
                } else {
                    self.state = MluState::Accumulate {
                        stop_time: event.time.clone() + self.prompt_window.inner().clone(),
                        cumulative: event.wire_pattern,
                    };
                    None
                }
            }
            MluState::Idle => {
                self.state = MluState::Accumulate {
                    stop_time: event.time.clone() + self.prompt_window.inner().clone(),
                    cumulative: event.wire_pattern,
                };
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn lookup_table_new() {
        let table = LookupTable::new();

        for n in 0..=u16::MAX {
            let pattern = WirePattern::from_bits(n);
            assert!(!table.contains(pattern));
        }
    }

    #[test]
    fn lookup_table_insert() {
        let mut table = LookupTable::new();

        assert!(!table.contains(WirePattern::from_bits(0)));
        assert!(table.insert(WirePattern::from_bits(0)));
        assert!(table.contains(WirePattern::from_bits(0)));
        assert!(!table.insert(WirePattern::from_bits(0)));
        assert!(table.contains(WirePattern::from_bits(0)));
    }

    #[test]
    fn lookup_table_remove() {
        let mut table = LookupTable::new();

        table.insert(WirePattern::from_bits(0));
        assert!(table.contains(WirePattern::from_bits(0)));
        assert!(table.remove(WirePattern::from_bits(0)));
        assert!(!table.contains(WirePattern::from_bits(0)));
        assert!(!table.remove(WirePattern::from_bits(0)));
    }

    #[test]
    fn lookup_table_from_iter() {
        let ps = [
            WirePattern::from_bits(1),
            WirePattern::from_bits(2),
            WirePattern::from_bits(2),
            WirePattern::from_bits(3),
        ];

        let table: LookupTable = ps.iter().copied().collect();

        for p in &ps {
            assert!(table.contains(*p));
        }
    }

    #[test]
    fn lookup_table_from_array() {
        let table = LookupTable::from([
            WirePattern::from_bits(0),
            WirePattern::from_bits(1),
            WirePattern::from_bits(2),
        ]);
        let unordered_table = LookupTable::from([
            WirePattern::from_bits(2),
            WirePattern::from_bits(1),
            WirePattern::from_bits(0),
            WirePattern::from_bits(2),
        ]);
        assert_eq!(table, unordered_table);
    }

    #[test]
    fn lookup_table_to_string() {
        let mut table = LookupTable::new();
        assert_eq!(table.to_string(), "");

        table.insert(WirePattern::from_bits(u16::MAX));
        assert_eq!(
            table.to_string(),
            "0xffff 1 XXXXXXXXXXXXXXXX, 16 bits, 1 clusters"
        );

        table.insert(WirePattern::from_bits(0));
        assert_eq!(
            table.to_string(),
            "0x0000 1 ................, 0 bits, 0 clusters
0xffff 1 XXXXXXXXXXXXXXXX, 16 bits, 1 clusters"
        );

        table.insert(WirePattern::from_bits(36449));
        assert_eq!(
            table.to_string(),
            "0x0000 1 ................, 0 bits, 0 clusters
0x8e61 1 X....XX..XXX...X, 7 bits, 3 clusters
0xffff 1 XXXXXXXXXXXXXXXX, 16 bits, 1 clusters"
        );
    }

    #[test]
    fn lookup_table_from_str() {
        let mut string = String::new();
        let mut table = LookupTable::new();
        assert_eq!(table, LookupTable::from_str(&string).unwrap());

        string.push_str("0x0000 1 ................, 0 bits, 0 clusters");
        table.insert(WirePattern::from_bits(0));
        assert_eq!(table, LookupTable::from_str(&string).unwrap());

        string.push_str("\n0x0000 1 ................, 0 bits, 0 clusters\n");
        assert_eq!(table, LookupTable::from_str(&string).unwrap());

        string.push_str("0xffff 1 XXXXXXXXXXXXXXXX, 16 bits, 1 clusters");
        table.insert(WirePattern::from_bits(u16::MAX));
        assert_eq!(table, LookupTable::from_str(&string).unwrap());

        assert_eq!(table, LookupTable::from_str(&table.to_string()).unwrap());
    }
}
