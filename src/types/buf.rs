use derive_more::Display;

use core::borrow::Borrow;

use crate::utils::u64_digits;

/// An buffer for building a in-line string representation of a timestamps, header, or subtitle entry without heap allocation.
#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{}", self.as_str())]
pub struct Buffer<const N: usize>([u8; N]);

impl<const N: usize> Borrow<str> for Buffer<N> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn borrow(&self) -> &str {
    self.as_str()
  }
}

impl<const N: usize> core::ops::Deref for Buffer<N> {
  type Target = str;

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn deref(&self) -> &Self::Target {
    self.borrow()
  }
}

impl<const N: usize> AsRef<str> for Buffer<N> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  fn as_ref(&self) -> &str {
    self
  }
}

impl<const N: usize> Buffer<N> {
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub(crate) const fn new() -> Self {
    Self([0; N])
  }

  /// Returns the length of the encoded timestamp string.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn len(&self) -> usize {
    match self.0.last() {
      Some(val) => *val as usize,
      None => panic!("buffer must be initialized with at least one byte to store the length"),
    }
  }

  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn incr_len(&mut self, incr: usize) {
    match self.0.last_mut() {
      Some(val) => *val = *val + incr as u8,
      None => panic!("buffer must be initialized with at least one byte to store the length"),
    }
  }

  #[cfg_attr(not(tarpaulin), inline(always))]
  pub(crate) const fn write_str(&mut self, bytes: &str) {
    self.write_bytes(bytes.as_bytes());
  }

  #[cfg_attr(not(tarpaulin), inline(always))]
  const fn write_bytes(&mut self, bytes: &[u8]) {
    let len = self.len();
    let src_len = bytes.len();
    assert!(len + src_len < N, "buffer overflow");
    unsafe {
      core::ptr::copy_nonoverlapping(
        bytes.as_ptr(),
        self.0.split_at_mut(len).1.as_mut_ptr(),
        src_len,
      );
    }
    self.incr_len(src_len);
  }

  #[cfg_attr(not(tarpaulin), inline(always))]
  pub(crate) const fn fmt_u64(&mut self, mut n: u64) {
    let mut buf = [0; 20];

    // Handle zero explicitly
    if n == 0 {
      self.write_str("0");
      return;
    }

    // 1. Calculate the length.
    // (You could easily swap this out for your `u64_digits` match statement!)
    let len = u64_digits(n);

    // 2. Extract digits right-to-left and convert to ASCII
    let mut i = len;
    while n > 0 {
      i -= 1;
      // Map the digit to its ASCII character representation
      buf[i] = (n % 10) as u8 + b'0';
      n /= 10;
    }

    self.write_bytes(buf.split_at(len).0);
  }

  /// Returns the string representation of this buffer, which is always valid UTF-8.
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &str {
    let len = self.len();
    // SAFETY: The buffer is always initialized with valid UTF-8 bytes, so this is safe.
    unsafe { core::str::from_utf8_unchecked(self.0.split_at(len).0) }
  }
}

#[test]
fn test_write_0() {
  let mut buf = Buffer::<10>::new();
  buf.fmt_u64(0);
  assert_eq!(buf.as_ref(), "0");
}

#[test]
#[should_panic]
fn test_len_panic() {
  let buf = Buffer::<0>::new();
  buf.len(); // 6 digits, but buffer can only hold 4 + length byte
}

#[test]
#[should_panic]
fn test_write_overflow() {
  let mut buf = Buffer::<0>::new();
  buf.incr_len(0);
}
