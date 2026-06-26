# periodic-map

A minimal, no_std-compatible Rust library for mapping indices to values based on a repeating pattern of valid and invalid windows.

## Overview

`PeriodicMap` divides the input space into repeating cycles, each consisting of a "valid window" followed by an "invalid window". Indices within the valid window map to incrementing values, while indices in the invalid window return `None`.

For instance with a valid window of 5 and an invalid window of 10 it will give these values:

| Indice Range | Result |
| --- | --- |
| 0..5 | Some(0) |
| 5..15   | None |
| 15.20  | Some(1) |
| 20..30 | None |
| 30..35 | Some(2) |
| 35..45 | None |
| keeps repeating. |




### Example

```rust
use periodic_map::PeriodicMap;

let periodic_map = PeriodicMap::new(5, 10).unwrap();

assert_eq!(periodic_map.map(0), Some(0));
assert_eq!(periodic_map.map(3), Some(0));
assert_eq!(periodic_map.map(5), None);
assert_eq!(periodic_map.map(17), Some(1));
assert_eq!(periodic_map.map(19), Some(1));
assert_eq!(periodic_map.map(29), None);
assert_eq!(periodic_map.map(34), Some(2));
```

## API

### Structs

- [`PeriodicMap`](src/lib.rs): Main struct for mapping indices.

### Methods

- [`PeriodicMap::new(valid_window, invalid_window)`](src/lib.rs):  
  Creates a new map. Both window sizes must be at least 1.

- [`PeriodicMap::map(index)`](src/lib.rs):  
  Returns `Some(period)` if the index is in a valid window, or `None` otherwise.

### Errors

- [`PeriodicMapError`](src/lib.rs):  
  - `ValidWindowLessThanOne`
  - `InvalidWindowLessThanOne`


## License

- MIT