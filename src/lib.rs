//! HistoryBuffer
//!
//! This crate provides a circular history buffer similar to what you would want for
//! a terminal window buffer. It returns bytes from any range still in memory.
//!
//! It copies new data into the circular buffer, automatically handling wrap-arounds,
//! always overwriting the oldest data. (Data is copied when entering and exiting)
//!
//! The size of the buffer is always scaled up to the next power of 2, avoiding many
//! unnecessary code branches.
//!
//! The library could be easily extended to other types besides [u8].
//!
//! ```rust
//! use historybuffer::HistoryBuffer;
//!
//! fn main() {
//!     let mut hb = HistoryBuffer::new(6); // Create an 8-element buffer (next power of 2).
//!
//!     hb.add("The Terminal History.".to_string().as_bytes());
//!     assert_eq!(
//!         hb.get_vec(15, 6),
//!         "story.".to_string().as_bytes().to_vec()
//!     );
//!
//!     assert_eq!(hb.last_byte(), Some(b'.'));
//!
//!     assert_eq!(hb.get_recent(4), "ory.".to_string().as_bytes());
//!
//!     assert_eq!(hb.get_index(), 13);
//!
//!     assert_eq!(hb.get(13), Some(b'H'));
//!
//!     assert_eq!(hb.get_last_index(), 20);
//!
//!     assert_eq!(hb.get(20), Some(b'.'));
//!
//!     hb.add(" and".to_string().as_bytes());
//!
//!     assert_eq!(hb.get(13), None);
//! }
//!```
//!
//!
//! get_vec returns as much data as possible from any desired range in history
//! up to a maximum length of the available data at that moment.
//!
//! get_vec_and_index returns a tuple containing the data and the start index of
//! the returned data.
//!
//! Note: This code has not been tested for wrapping `usize` values > 4 billion chars 
//!       from long running apps.


//
// A tale of two indices
//
// Data is stored in a block whose length is always a power of 2 (2, 4, 8, ...)
// Data is aligned according to the equation:
//
// buf_index = (start_index & mask)
//
// self.next_index points to where the next byte will go, which is also the
// oldest byte's index if the buffer is full.
//

#[derive(Default)]
pub struct HistoryBuffer {
    buf: Vec<u8>,
    len: usize,
    mask: usize,
    next_running: usize,
}

impl HistoryBuffer {
    pub fn new(min_buf_size: usize) -> Self {
        let power_two_size = next_power_of_two(min_buf_size).clamp(2, 1 << 23);
        Self {
            buf: vec![0; power_two_size],
            mask: power_two_size - 1,
            ..Default::default()
        }
    }

    /// add
    ///
    /// This function ingests data slices and copies them to the internal buffer.
    pub fn add(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        let buf_size = self.buf.len();
        self.next_running += data.len();
        self.len = (self.len + data.len()).min(buf_size);
        let inndx = (self.next_running - data.len().min(buf_size)) & self.mask;
        let outdx = self.next_running & self.mask;

        if outdx > inndx {
            let num = outdx.saturating_sub(inndx);
            self.buf[inndx..outdx].copy_from_slice(&data[0..num]);
        } else {
            let num = (outdx + buf_size).saturating_sub(inndx);
            let wrap = data.len().saturating_sub(outdx);
            self.buf[inndx..buf_size].copy_from_slice(&data[(data.len() - num)..wrap]);
            self.buf[0..outdx].copy_from_slice(&data[wrap..data.len()]);
        }
    }

    /// clear the buffer
    ///
    /// Clear the len - Data is still there, but access is denied.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// clear_at the buffer at a specific point
    ///
    /// Set the len to erase all history before start_index.
    /// Data is still there, but access is denied.
    pub fn clear_at(&mut self, new_start_index: usize) {
        self.len = self.next_running.saturating_sub(new_start_index).min(self.len);
    }

    /// get
    ///
    /// Gets the byte value at index.
    pub fn get(&self, index: usize) -> Option<u8> {
        if (index >= self.next_running - self.len) && (index < self.next_running) {
            Some(self.buf[index & self.mask])
        } else {
            None
        }
    }

    /// get_index
    ///
    /// Gets the history's starting index, or oldest byte
    pub fn get_index(&self) -> usize {
        self.next_running - self.len
    }

    /// get_last_index
    ///
    /// Gets the index of the most recent addtion.
    pub fn get_last_index(&self) -> usize {
        self.next_running.saturating_sub(1).max(self.next_running.saturating_sub(self.len))
    }

    /// get_len
    ///
    /// Gets the current length of data in the buffer.
    pub fn get_len(&self) -> usize {
        self.len
    }

    /// get_recent
    ///
    /// Returns the most recent bytes up to max_len.
    pub fn get_recent(&self, max_len: usize) -> Vec<u8> {
        let len = max_len.min(self.len);
        let start = self.next_running - len;
        let (v, _start_index) = self.get_vec_and_index(start, len);
        v
    }

    /// get_vec
    ///
    /// Returns a history-filled vector without any index.
    ///
    /// Note: If you request the full buffer length and nothing extra,
    /// you can tell that your data has shifted by looking at the vector's length:
    ///
    /// Example:
    /// ```rust
    /// use historybuffer::HistoryBuffer;
    ///
    /// let mut hb = HistoryBuffer::new(8);
    /// hb.add("The Terminal History.".to_string().as_bytes());
    /// let history = hb.get_vec(13, 8);
    /// if history.len() < 8 {
    ///     println!("...");
    /// }
    ///
    /// assert_eq!(
    ///     hb.get_vec(15, 6),
    ///     "story.".to_string().as_bytes().to_vec()
    /// );
    /// ```
    pub fn get_vec(&self, start_index: usize, max_len: usize) -> Vec<u8> {
        let (v, _start_idx) = self.get_vec_and_index(start_index, max_len);
        v
    }

    /// get_vec_and_index
    ///
    /// Returns a history vector along with the starting index.
    ///
    /// Note: The function may return less data than requested if the data has been overwritten,
    /// or the buffer has been cleared or shortened.
    ///
    /// Example:
    /// ```rust
    /// use historybuffer::HistoryBuffer;
    ///
    /// fn main() {
    ///     let mut hb = HistoryBuffer::new(6); // Create an 8-element buffer (next power of 2).
    ///     hb.add("The Terminal History.".to_string().as_bytes());
    ///
    ///     assert_eq!(
    ///         hb.get_vec_and_index(0, 100000),
    ///         ("History.".to_string().as_bytes().to_vec(), 13usize)
    ///     );
    ///
    ///     // Is the same as:
    ///     assert_eq!(
    ///         hb.get_vec_and_index(13, 8),
    ///         ("History.".to_string().as_bytes().to_vec(), 13usize)
    ///     );
    ///
    ///     // But it changes if more text is added:
    ///
    ///     hb.add(" and".to_string().as_bytes());
    ///
    ///     assert_eq!(
    ///         hb.get_vec_and_index(0, 100000),
    ///         ("ory. and".to_string().as_bytes().to_vec(), 17usize)
    ///     );
    ///
    ///     assert_eq!(
    ///         hb.get_vec_and_index(13, 8),
    ///         ("ory.".to_string().as_bytes().to_vec(), 17usize)
    ///     );
    /// }
    /// ```
    pub fn get_vec_and_index(&self, start_index: usize, max_len: usize) -> (Vec<u8>, usize) {
        let buf_size = self.buf.len();

        let out = (start_index + max_len).min(self.next_running);
        let inn = (start_index.max(self.next_running - self.len)).min(out);
        let inndx = inn & self.mask;
        let outdx = out & self.mask;
        let num = out.saturating_sub(inn);
        let mut v = Vec::with_capacity(num); // NOTE: Vec's need to already have FILLED vectors for .copy_from_slice to work
        v.resize(num, 0);
        if num == 0 {
            return (v, 0);
        }

        if outdx > inndx {
            v[0..num].copy_from_slice(&self.buf[inndx..outdx]);
        } else {
            v[0..(buf_size - inndx)].copy_from_slice(&self.buf[inndx..buf_size]);
            v[(buf_size - inndx)..num].copy_from_slice(&self.buf[0..outdx]);
        }
        (v, inn)
    }

    /// last_byte
    ///
    /// Returns the most recent byte added to the buffer.
    pub fn last_byte(&self) -> Option<u8> {
        if self.len > 0 {
            let idx = (self.next_running + self.mask) & self.mask; // Go back by going forward mask bytes (mask = buf.len() - 1)
            Some(self.buf[idx])
        } else {
            None
        }
    }
}

fn next_power_of_two(n: usize) -> usize {
    let mut power = 1;
    while power < n && power < (1 << 30) {
        // Limit to ~1 TB
        power <<= 1; // Power = 2 * Power
    }
    power
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_add_with_vectors() {
        let test_vectors: [(
            &str,
            &[u8],
            Vec<u8>,
            usize,
            usize,
            Option<u8>,
            Option<u8>,
            usize,
            usize,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
        ); 8] = [
            (
                "",
                b"",
                vec![0; 8],
                0,
                0,
                None,
                None,
                0,
                0,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            (
                "Empty Add",
                b"",
                vec![0; 8],
                0,
                0,
                None,
                None,
                0,
                0,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            (
                "First H",
                b"H",
                vec![b'H', 0, 0, 0, 0, 0, 0, 0],
                0,
                1,
                Some(b'H'),
                None,
                0,
                0,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            (
                "Next HelloO",
                b"ellooO",
                vec![b'H', b'e', b'l', b'l', b'o', b'o', b'O', 0],
                0,
                7,
                Some(b'O'),
                None,
                0,
                6,
                vec![b'e', b'l', b'l', b'o', b'o', b'O'],
                vec![b'O'],
                vec![],
                vec![],
            ),
            (
                "Filling K",
                b"K",
                vec![b'H', b'e', b'l', b'l', b'o', b'o', b'O', b'K'],
                0,
                8,
                Some(b'K'),
                None,
                0,
                7,
                vec![b'e', b'l', b'l', b'o', b'o', b'O', b'K'],
                vec![b'O', b'K'],
                vec![b'K'],
                vec![],
            ),
            (
                "Overwriting J",
                b"J",
                vec![b'J', b'e', b'l', b'l', b'o', b'o', b'O', b'K'],
                1,
                8,
                Some(b'J'),
                Some(b'J'),
                1,
                8,
                vec![b'e', b'l', b'l', b'o', b'o', b'O', b'K', b'J'],
                vec![b'O', b'K', b'J'],
                vec![b'K', b'J'],
                vec![],
            ),
            (
                "Long Long Add",
                b"Second Pa A Really Long Addition",
                vec![b'n', b'A', b'd', b'd', b'i', b't', b'i', b'o'],
                33,
                8,
                Some(b'n'),
                None,
                33,
                40,
                vec![],
                vec![],
                vec![],
                vec![b'o', b'n'],
            ),
            (
                "After Overwrite O",
                b"P",
                vec![b'n', b'P', b'd', b'd', b'i', b't', b'i', b'o'],
                34,
                8,
                Some(b'P'),
                None,
                34,
                41,
                vec![],
                vec![],
                vec![],
                vec![b'o', b'n', b'P'],
            ),
        ];

        let mut tbuf = HistoryBuffer::new(5);

        for (
            test_name,
            input,
            exbuf,
            exrunning,
            exlen,
            exlast_byte,
            exget_byte8,
            exget_index,
            exlast_index,
            v1,
            v2,
            v3,
            v4,
        ) in test_vectorsget
        {
            println!("\n\n\t\t\t{}:  Adding: {:#?}", test_name, input);
            tbuf.add(input);
            //tbuf.print_buffer();

            println!("        .buf:");
            assert_eq!(tbuf.buf, exbuf);
            println!("        .next_running:");
            assert_eq!(tbuf.get_index(), exrunning);
            println!("        .len:");
            assert_eq!(tbuf.len, exlen);

            println!("        .last_byte:");
            assert_eq!(tbuf.last_byte(), exlast_byte);
            println!("        .get(8):");
            assert_eq!(tbuf.get(8), exget_byte8);

            println!("        .get_index:");
            assert_eq!(tbuf.get_index(), exget_index);
            println!("        .last_index:");
            assert_eq!(tbuf.get_last_index(), exlast_index);

            println!("        v1 get_vec(1, 8):");
            assert_eq!(tbuf.get_vec(1, 8), v1);
            assert_eq!(tbuf.get_vec(1, 9), v1);
            assert_eq!(tbuf.get_vec(1, 10), v1);
            println!("        v2 get_vec(6, 3):");
            assert_eq!(tbuf.get_vec(6, 3), v2);
            println!("        v3 get_vec(7, 3):");
            assert_eq!(tbuf.get_vec(7, 3), v3);
            println!("        v4 get_vec(39, 3):");
            assert_eq!(tbuf.get_vec(39, 3), v4);
        }
        println!("  Future Proofing:");
        assert_eq!(
            tbuf.get_vec(0, 100),
            vec![b'd', b'd', b'i', b't', b'i', b'o', b'n', b'P',]
        );
    }

    #[test]
    fn test_after_clear() {
        let test_vectors: [(
            &str,
            &[u8],
            usize,
            usize,
            Option<u8>,
            usize,
            usize,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
        ); 8] = [
            ("", b"", 0, 0, None, 0, 0, vec![], vec![], vec![], vec![]),
            (
                "Empty Add",
                b"",
                0,
                0,
                None,
                0,
                0,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            (
                "First H",
                b"H",
                0,
                1,
                Some(b'H'),
                0,
                0,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            (
                "Next HelloO",
                b"ellooO",
                0,
                7,
                Some(b'O'),
                0,
                6,
                vec![b'e', b'l', b'l', b'o', b'o', b'O'],
                vec![b'O'],
                vec![],
                vec![],
            ),
            (
                "Filling K",
                b"K",
                0,
                8,
                Some(b'K'),
                0,
                7,
                vec![b'e', b'l', b'l', b'o', b'o', b'O', b'K'],
                vec![b'O', b'K'],
                vec![b'K'],
                vec![],
            ),
            (
                "Overwriting J",
                b"J",
                1,
                8,
                Some(b'J'),
                1,
                8,
                vec![b'e', b'l', b'l', b'o', b'o', b'O', b'K', b'J'],
                vec![b'O', b'K', b'J'],
                vec![b'K', b'J'],
                vec![],
            ),
            (
                "Long Long Add",
                b"Second Pa A Really Long Addition",
                33,
                8,
                Some(b'n'),
                33,
                40,
                vec![],
                vec![],
                vec![],
                vec![b'o', b'n'],
            ),
            (
                "After Overwrite O",
                b"P",
                34,
                8,
                Some(b'P'),
                34,
                41,
                vec![],
                vec![],
                vec![],
                vec![b'o', b'n', b'P'],
            ),
        ];

        let mut tbuf = HistoryBuffer::new(5);
        for i in 0..9 {
            let vecu8 = std::iter::repeat(b'X').take(i).collect::<Vec<u8>>();
            tbuf.add(&vecu8);
            tbuf.clear();
            let offset = tbuf.get_index();
            println!("======================================");
            println!("==== i {} offset: {}  ========", i, offset);

            for (
                test_name,
                input,
                exrunning,
                exlen,
                exlast_byte,
                exstart_index,
                exlast_index,
                v1,
                v2,
                v3,
                v4,
            ) in &test_vectors
            {
                println!("\n\n\t\t\t{}:  Adding: {:#?}", test_name, input);
                tbuf.add(input);
                //tbuf.print_buffer();

                //println!("        .buf:");
                //assert_eq!(tbuf.buf, exbuf);
                println!("        .next_running:");
                assert_eq!(tbuf.get_index(), exrunning + offset);
                println!("        .len:");
                assert_eq!(tbuf.len, *exlen);

                println!("        .last_byte():");
                assert_eq!(tbuf.last_byte(), *exlast_byte);

                println!("        .get_index():");
                assert_eq!(tbuf.get_index(), exstart_index + offset);
                println!("        .last_index:");
                assert_eq!(tbuf.get_last_index(), exlast_index + offset);

                println!("        v1 get_vec(1, 8):");
                assert_eq!(tbuf.get_vec(1 + offset, 8), *v1);
                assert_eq!(tbuf.get_vec(1 + offset, 9), *v1);
                assert_eq!(tbuf.get_vec(1 + offset, 10), *v1);
                println!("        v2 get_vec(6, 3):");
                assert_eq!(tbuf.get_vec(6 + offset, 3), *v2);
                println!("        v3 get_vec(7, 3):");
                assert_eq!(tbuf.get_vec(7 + offset, 3), *v3);
                println!("        v4 get_vec(39, 3):");
                assert_eq!(tbuf.get_vec(39 + offset, 3), *v4);
            }
            println!("  Future Proofing:");
            assert_eq!(
                tbuf.get_vec(0, 100000),
                vec![b'd', b'd', b'i', b't', b'i', b'o', b'n', b'P',]
            );
        }
    }

    #[test]
    fn test_history_buffer() {
        let mut tbuf = HistoryBuffer::new(6); // Create an 8-element buffer (next power of 2).
        let vecu8 = "The Terminal History.".to_string();
        tbuf.add(vecu8.as_bytes());
        print!("A:");
        assert_eq!(tbuf.last_byte(), Some(b'.'));
        print!("B:");
        assert_eq!(
            tbuf.get_vec_and_index(0, 100000),
            ("History.".to_string().as_bytes().to_vec(), 13usize)
        );
        print!("C:");
        assert_eq!(
            tbuf.get_vec_and_index(13, 8),
            ("History.".to_string().as_bytes().to_vec(), 13usize)
        );
        print!("E:");
        assert_eq!(tbuf.get_recent(4), "ory.".to_string().as_bytes());
        print!("H:");
        assert_eq!(tbuf.get_index(), 13);
        print!("I:");
        assert_eq!(tbuf.get(13), Some(b'H'));
        print!("J:");
        assert_eq!(tbuf.get_last_index(), 20);
        print!("K:");
        assert_eq!(tbuf.get(20), Some(b'.'));

        tbuf.add(" and".to_string().as_bytes());
        print!("P:");
        assert_eq!(tbuf.get(13), None);
        print!("Q:");
        assert_eq!(
            tbuf.get_vec_and_index(0, 100000),
            ("ory. and".to_string().as_bytes().to_vec(), 17usize)
        );
        print!("R:");
        assert_eq!(
            tbuf.get_vec_and_index(13, 8),
            ("ory.".to_string().as_bytes().to_vec(), 17usize)
        );
    }
}
