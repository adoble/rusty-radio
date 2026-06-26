#![cfg_attr(not(test), no_std)]
//!  Maps indices to values based on a repeating pattern of valid and invalid windows.
#[derive(Debug, PartialEq, Eq)]
pub struct PeriodicMap {
    valid_window: usize,
    invalid_window: usize,
}

/// Creates a new `PeriodicMap` with the specified valid and invalid window sizes.
///
/// The valid window determines how many consecutive indices are considered valid in each period,
/// while the invalid window determines how many consecutive indices are considered invalid.
/// The pattern repeats indefinitely: a block of valid indices followed by a block of invalid indices.
///
/// # Errors
///
/// Returns an error if either `valid_window` or `invalid_window` is less than 1.
///
/// # Example
/// ```
/// # use periodic_map::PeriodicMap;
/// let periodic_map = PeriodicMap::new(5, 10).unwrap();
/// ```
impl PeriodicMap {
    pub fn new(
        valid_window: usize,
        invalid_window: usize,
    ) -> Result<PeriodicMap, PeriodicMapError> {
        if valid_window < 1 {
            Err(PeriodicMapError::ValidWindowLessThanOne)?;
        }
        if invalid_window < 1 {
            Err(PeriodicMapError::InvalidWindowLessThanOne)?;
        }

        Ok(PeriodicMap {
            valid_window,
            invalid_window,
        })
    }

    ///
    /// This function divides the input space into repeating cycles of the sum of valid and invalid window.
    /// Within each cycle, the first valid window positions map to incrementing values
    /// (0, 1, 2, ...), while the remaining invalid window positions return `None`.
    ///
    /// # Pattern
    ///
    /// With `self.valid_window = 5` and `self.invalid_window = 10`:
    /// - Indices 0-4 → Some(0)
    /// - Indices 5-14 → None as this is an invalid window
    /// - Indices 15-19 → Some(1)
    /// - Indices 20-29 → None (invalid window)
    /// - Indices 30-34 → Some(2)
    /// - Indices 35-44 → None (invalid window)
    /// - Pattern repeats...
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use periodic_map::PeriodicMap;
    /// let periodic_map = PeriodicMap::new(5, 10).unwrap();
    /// assert_eq!(periodic_map.map(0), Some(0));
    /// assert_eq!(periodic_map.map(3), Some(0));
    /// assert_eq!(periodic_map.map(17), Some(1));
    /// assert_eq!(periodic_map.map(19), Some(1));
    /// assert_eq!(periodic_map.map(34), Some(2));
    /// assert_eq!(periodic_map.map(5), None);
    /// assert_eq!(periodic_map.map(29), None);
    /// assert_eq!(periodic_map.map(40), None);
    /// ```

    pub fn map(&self, index: usize) -> Option<usize> {
        let full_window = self.valid_window + self.invalid_window;
        let p = index / full_window;

        // As FULL_WINDOW is always possible this can never return None
        let r = index.checked_rem(full_window).unwrap();

        if r < self.valid_window { Some(p) } else { None }
    }

    /// Returns the index in the middle of the valid window for a given position.
    ///
    /// Given a `position` (the value returned by `map` for a valid index), this function
    /// computes the middle index within the corresponding valid window. The returned index
    /// is always within the valid window for that period.
    ///
    /// # Examples
    /// ```
    /// # use periodic_map::PeriodicMap;
    /// let periodic_map = PeriodicMap::new(5, 10).unwrap();
    /// assert_eq!(periodic_map.inverse_map(0), 2);
    /// assert_eq!(periodic_map.inverse_map(1), 17);
    /// assert_eq!(periodic_map.inverse_map(2), 32);
    /// ```
    pub fn inverse_map(&self, position: usize) -> usize {
        position * (self.valid_window + self.invalid_window) + 2
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum PeriodicMapError {
    ValidWindowLessThanOne,
    InvalidWindowLessThanOne,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_WINDOW: usize = 5;
    const INVALID_WINDOW: usize = 10;

    #[test]
    fn numbers_point_on() {
        let periodic_map: PeriodicMap = PeriodicMap::new(VALID_WINDOW, INVALID_WINDOW).unwrap();
        let mut i = 2;

        assert_eq!(periodic_map.map(i), Some(0));

        i = 17;
        assert_eq!(periodic_map.map(i), Some(1));

        i = 32;
        assert_eq!(periodic_map.map(i), Some(2));
    }

    #[test]
    fn test_in_valid_window() {
        let periodic_map: PeriodicMap = PeriodicMap::new(VALID_WINDOW, INVALID_WINDOW).unwrap();

        let mut i = 0;
        assert_eq!(periodic_map.map(i), Some(0));
        i = 1;
        assert_eq!(periodic_map.map(i), Some(0));
        i = 3;
        assert_eq!(periodic_map.map(i), Some(0));
        i = 4;
        assert_eq!(periodic_map.map(i), Some(0));

        i = 15;
        assert_eq!(periodic_map.map(i), Some(1));
        i = 16;
        assert_eq!(periodic_map.map(i), Some(1));
        i = 18;
        assert_eq!(periodic_map.map(i), Some(1));
        i = 19;
        assert_eq!(periodic_map.map(i), Some(1));

        assert_eq!(periodic_map.map(30), Some(2));
        assert_eq!(periodic_map.map(31), Some(2));
        assert_eq!(periodic_map.map(33), Some(2));
        assert_eq!(periodic_map.map(34), Some(2));
    }

    #[test]
    fn test_invalid_window() {
        let periodic_map: PeriodicMap = PeriodicMap::new(VALID_WINDOW, INVALID_WINDOW).unwrap();

        for i in 5..15 {
            assert!(periodic_map.map(i).is_none());
        }

        for i in 20..30 {
            assert!(periodic_map.map(i).is_none());
        }

        for i in 35..45 {
            assert!(periodic_map.map(i).is_none());
        }
    }

    #[test]
    fn test_new() {
        let p = PeriodicMap::new(4, 10).unwrap();

        assert_eq!(p.map(0), Some(0));

        let p = PeriodicMap::new(0, 10);

        assert_eq!(p, Err(PeriodicMapError::ValidWindowLessThanOne));

        let p = PeriodicMap::new(5, 0);

        assert_eq!(p, Err(PeriodicMapError::InvalidWindowLessThanOne));
    }

    #[test]
    fn test_inverse() {
        let p = PeriodicMap::new(5, 10).unwrap();

        assert_eq!(2, p.inverse_map(0));
        assert_eq!(17, p.inverse_map(1));
        assert_eq!(32, p.inverse_map(2));
    }
}
