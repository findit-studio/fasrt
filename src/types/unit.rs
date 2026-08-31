use derive_more::{Display, From, Into};

use core::str::FromStr;

use crate::error::*;

use super::macros::*;

/// The minute component (0–59) of a timestamp.
#[derive(
  Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, From, Into,
)]
#[display("{}", self.as_str())]
#[repr(transparent)]
pub struct Minute(pub(crate) u8);

impl FromStr for Minute {
  type Err = ParseMinuteError;

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    minute_from_str!(s)
  }
}

impl Minute {
  /// Create a new `Minute` with value 0.
  ///
  /// ```rust
  /// use fasrt::types::Minute;
  ///
  /// let minute = Minute::new();
  /// assert_eq!(minute.as_str(), "00");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new() -> Self {
    Self::with(0)
  }

  /// Create a new `Minute` from a `u8`.
  ///
  /// # Panics
  /// Panics if the value is greater than 59.
  ///
  /// ```rust
  /// use fasrt::types::Minute;
  ///
  /// let minute = Minute::with(30);
  /// assert_eq!(minute.as_str(), "30");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with(value: u8) -> Self {
    if value > 59 {
      panic!("Minute value must be between 0-59");
    }
    Self(value)
  }

  /// Try to create a new `Minute` from a `u8`, returning `None` if the value is out of range.
  ///
  /// ```rust
  /// use fasrt::types::Minute;
  ///
  /// assert_eq!(Minute::try_with(30), Some(Minute::with(30)));
  /// assert_eq!(Minute::try_with(60), None);
  /// ```
  pub const fn try_with(value: u8) -> Option<Self> {
    if value > 59 { None } else { Some(Self(value)) }
  }

  /// Returns the string representation of this `Minute`, zero-padded to 2 digits.
  ///
  /// ```rust
  /// use fasrt::types::Minute;
  ///
  /// let minute = Minute::with(5);
  /// assert_eq!(minute.as_str(), "05");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'static str {
    minute_to_str!(self.0)
  }
}

/// The second component (0–59) of a timestamp.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, Into)]
#[display("{}", self.as_str())]
#[repr(transparent)]
pub struct Second(pub(crate) u8);

impl FromStr for Second {
  type Err = ParseSecondError;

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    second_from_str!(s)
  }
}

impl Second {
  /// Create a new `Second` with value 0.
  ///
  /// ```rust
  /// use fasrt::types::Second;
  ///
  /// let second = Second::new();
  /// assert_eq!(second.as_str(), "00");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new() -> Self {
    Self::with(0)
  }

  /// Create a new `Second` from a `u8`.
  ///
  /// # Panics
  /// Panics if the value is greater than 59.
  ///
  /// ```rust
  /// use fasrt::types::Second;
  ///
  /// let second = Second::with(30);
  /// assert_eq!(second.as_str(), "30");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with(value: u8) -> Self {
    if value > 59 {
      panic!("Second value must be between 0-59");
    }
    Self(value)
  }

  /// Try to create a new `Second` from a `u8`, returning `None` if the value is out of range.
  ///
  /// ```rust
  /// use fasrt::types::Second;
  ///
  /// assert_eq!(Second::try_with(30), Some(Second::with(30)));
  /// assert_eq!(Second::try_with(60), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn try_with(value: u8) -> Option<Self> {
    if value > 59 { None } else { Some(Self(value)) }
  }

  /// Returns the string representation of this `Second`, zero-padded to 2 digits.
  ///
  /// ```rust
  /// use fasrt::types::Second;
  ///
  /// let second = Second::with(5);
  /// assert_eq!(second.as_str(), "05");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'static str {
    second_to_str!(self.0)
  }
}

/// The millisecond component (0–999) of a timestamp.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, Into)]
#[display("{}", self.as_str())]
#[repr(transparent)]
pub struct Millisecond(pub(crate) u16);

impl FromStr for Millisecond {
  type Err = ParseMillisecondError;

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    millisecond_from_str!(s)
  }
}

impl Millisecond {
  /// Create a new `Millisecond` with value 0.
  ///
  /// ```rust
  /// use fasrt::types::Millisecond;
  ///
  /// let ms = Millisecond::new();
  /// assert_eq!(ms.as_str(), "000");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new() -> Self {
    Self::with(0)
  }

  /// Create a new `Millisecond` from a `u16`.
  ///
  /// # Panics
  /// Panics if the value is greater than 999.
  ///
  /// ```rust
  /// use fasrt::types::Millisecond;
  ///
  /// let ms = Millisecond::with(500);
  /// assert_eq!(ms.as_str(), "500");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with(value: u16) -> Self {
    if value > 999 {
      panic!("Millisecond value must be between 0-999");
    }
    Self(value)
  }

  /// Try to create a new `Millisecond` from a `u16`, returning `None` if the value is out of range.
  ///
  /// ```rust
  /// use fasrt::types::Millisecond;
  ///
  /// assert_eq!(Millisecond::try_with(500), Some(Millisecond::with(500)));
  /// assert_eq!(Millisecond::try_with(1000), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn try_with(value: u16) -> Option<Self> {
    if value > 999 { None } else { Some(Self(value)) }
  }

  /// Returns the string representation of this `Millisecond`, zero-padded to 3 digits.
  ///
  /// ```rust
  /// use fasrt::types::Millisecond;
  ///
  /// let ms = Millisecond::with(5);
  /// assert_eq!(ms.as_str(), "005");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'static str {
    millisecond_to_str!(self.0)
  }
}

/// The centisecond component (0–99) of a timestamp.
///
/// Centiseconds are the sub-second unit of ASS/SSA timestamps (`H:MM:SS.cc`),
/// where the fractional part is always exactly two digits.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, Into)]
#[display("{}", self.as_str())]
#[repr(transparent)]
pub struct Centisecond(pub(crate) u8);

impl FromStr for Centisecond {
  type Err = ParseCentisecondError;

  #[cfg_attr(not(tarpaulin), inline(always))]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    centisecond_from_str!(s)
  }
}

impl Centisecond {
  /// Create a new `Centisecond` with value 0.
  ///
  /// ```rust
  /// use fasrt::types::Centisecond;
  ///
  /// let cs = Centisecond::new();
  /// assert_eq!(cs.as_str(), "00");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn new() -> Self {
    Self::with(0)
  }

  /// Create a new `Centisecond` from a `u8`.
  ///
  /// # Panics
  /// Panics if the value is greater than 99.
  ///
  /// ```rust
  /// use fasrt::types::Centisecond;
  ///
  /// let cs = Centisecond::with(50);
  /// assert_eq!(cs.as_str(), "50");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn with(value: u8) -> Self {
    if value > 99 {
      panic!("Centisecond value must be between 0-99");
    }
    Self(value)
  }

  /// Try to create a new `Centisecond` from a `u8`, returning `None` if the
  /// value is out of range.
  ///
  /// ```rust
  /// use fasrt::types::Centisecond;
  ///
  /// assert_eq!(Centisecond::try_with(50), Some(Centisecond::with(50)));
  /// assert_eq!(Centisecond::try_with(100), None);
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn try_with(value: u8) -> Option<Self> {
    if value > 99 { None } else { Some(Self(value)) }
  }

  /// Returns the string representation of this `Centisecond`, zero-padded to
  /// 2 digits.
  ///
  /// ```rust
  /// use fasrt::types::Centisecond;
  ///
  /// let cs = Centisecond::with(5);
  /// assert_eq!(cs.as_str(), "05");
  /// ```
  #[cfg_attr(not(tarpaulin), inline(always))]
  pub const fn as_str(&self) -> &'static str {
    centisecond_to_str!(self.0)
  }
}

#[test]
#[should_panic]
fn minute_panic() {
  let _ = Minute::with(60);
}

#[test]
#[should_panic]
fn centisecond_panic() {
  let _ = Centisecond::with(100);
}

#[test]
#[should_panic]
fn second_panic() {
  let _ = Second::with(60);
}

#[test]
#[should_panic]
fn millisecond_panic() {
  let _ = Millisecond::with(1000);
}
