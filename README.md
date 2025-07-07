# History Buffer

## What is this?

History Buffer is a byte-based ring history buffer cache. It efficiently copies data passed to it into a ring buffer, where old data is overwritten as new data arrives.

The buffer provides flexible access to stored data by tracking the byte index of all entries, allowing you to retrieve exactly the data you want. You can also request data 
without the index values for a simpler interface.

Data is copied upon ingestion and again when requested. New data automatically overwrites old data.

**Note**: For more efficient processing, the buffer size is always increased to the next power of 2. A buffer size of 8 remains 8, 9 becomes 16, 129 becomes 256.

**Note**: This software has not tested usize wrap around indexing where the index value wraps.

## Example Usage

```rust
use historybuffer::HistoryBuffer;

fn main() {
    let mut hb = HistoryBuffer::new(6); // Create an 8-element buffer (next power of 2).

    hb.add("The Terminal History.".to_string().as_bytes());

    assert_eq!(
        hb.get_vec_and_index(0, 100000),
        ("History.".to_string().as_bytes().to_vec(), 13usize)
    );

    assert_eq!(
        hb.get_vec(15, 6),
        "story.".to_string().as_bytes().to_vec()
    );

    assert_eq!(hb.last_byte(), Some(b'.'));

    assert_eq!(hb.get_recent(4), "ory.".to_string().as_bytes());

    assert_eq!(hb.get_index(), 13);

    assert_eq!(hb.get(13), Some(b'H'));

    assert_eq!(hb.get_last_index(), 20);

    assert_eq!(hb.get(20), Some(b'.'));

    hb.add(" and".to_string().as_bytes());

    assert_eq!(hb.get(13), None);
}
```

## Features

- **Ring Buffer Implementation**: Efficiently manages data with automatic overwriting of old entries.
- **Index Tracking**: Allows precise retrieval of data by byte index.
- **Flexible Access Methods**: Provides methods to get data with or without indices for different use cases.

# Installation

Add `historybuffer` to your `Cargo.toml`:

```toml
[dependencies]
historybuffer = "0.1.0"
```

Then, include it in your Rust project:

```rust
use historybuffer::HistoryBuffer;
```
