//! Simple buffered writer.

use std::io::Write;

/// Buffered writer.
/// Wont flush automatically, you will need to call `flush` manually.
pub struct BufWriter<T: Write> {
    inner: T,
    buf: Vec<u8>,
}

impl<T: Write> BufWriter<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            buf: Vec::new(),
        }
    }
}

impl<T: Write> Write for BufWriter<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.write_all(&self.buf)?;
        self.buf.clear();
        self.inner.flush()
    }
}
