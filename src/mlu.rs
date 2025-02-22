use crate::gen::WirePattern;

/// Set of [`WirePattern`]s.
///
/// The [`LookupTable`] determines the set of wire patterns of interest that
/// produce a TRG signal out of the MLU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LookupTable {
    inner: [bool; 2usize.pow(16)],
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
            inner: [false; 2usize.pow(16)],
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
