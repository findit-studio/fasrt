/// Returns the number of digits in the decimal representation of `n`.
#[cfg_attr(not(tarpaulin), inline(always))]
pub const fn u64_digits(n: u64) -> usize {
  if n == 0 { 1 } else { (n.ilog10() + 1) as usize }
}

/// A line iterator that yields lines without the line terminator.
///
/// Because every yielded line is a subslice of the original input, callers
/// can recover a line's byte offset with pointer arithmetic against the
/// input, which the WebVTT and ASS/SSA parsers use to build multi-line
/// bodies without allocating.
///
/// Handles all three line terminator styles: `\n` (LF), `\r\n` (CRLF),
/// and standalone `\r` (CR).
pub(crate) struct Lines<'a> {
  input: &'a str,
  pos: usize,
}

impl<'a> Lines<'a> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub(crate) const fn new(input: &'a str) -> Self {
    Self { input, pos: 0 }
  }
}

impl<'a> Iterator for Lines<'a> {
  type Item = &'a str;

  fn next(&mut self) -> Option<Self::Item> {
    if self.pos >= self.input.len() {
      return None;
    }

    let bytes = &self.input.as_bytes()[self.pos..];

    #[cfg(all(feature = "memchr", not(miri)))]
    let found = memchr::memchr2(b'\n', b'\r', bytes);
    #[cfg(any(not(feature = "memchr"), miri))]
    let found = bytes.iter().position(|&b| b == b'\n' || b == b'\r');

    match found {
      Some(i) => {
        let line = &self.input[self.pos..self.pos + i];
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
          self.pos += i + 2;
        } else {
          self.pos += i + 1;
        }
        Some(line)
      }
      None => {
        let line = &self.input[self.pos..];
        self.pos = self.input.len();
        Some(line)
      }
    }
  }
}
