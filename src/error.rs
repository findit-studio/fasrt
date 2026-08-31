use core::num::ParseIntError;
use derive_more::{IsVariant, TryUnwrap, Unwrap};

/// The error type for parsing minute components of timestamps.
#[derive(Debug, Clone, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap, thiserror::Error)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum ParseMinuteError {
  /// The minute component is not zero-padded to 2 digits.
  #[error("minute component is not zero-padded to 2 digits")]
  #[unwrap(ignore)]
  #[try_unwrap(ignore)]
  NotPadded,
  /// The minute component is out of range (not between 0-59).
  #[error("minute component must be between 0-59, but was {0}")]
  Overflow(u8),
  /// Not a valid number.
  #[error(transparent)]
  ParseInt(#[from] ParseIntError),
}

/// The error type for parsing second components of timestamps.
#[derive(Debug, Clone, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap, thiserror::Error)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum ParseSecondError {
  /// The second component is not zero-padded to 2 digits.
  #[error("second component is not zero-padded to 2 digits")]
  #[unwrap(ignore)]
  #[try_unwrap(ignore)]
  NotPadded,
  /// The second component is out of range (not between 0-59).
  #[error("second component must be between 0-59, but was {0}")]
  Overflow(u8),
  /// Not a valid number.
  #[error(transparent)]
  ParseInt(#[from] ParseIntError),
}

/// The error type for parsing hour components of timestamps.
///
/// This enum is shared by the SRT, WebVTT and ASS/SSA parsers:
/// - **SRT** hours are 2–3 digits (0–999): uses [`NotPadded`](Self::NotPadded)
///   and [`Overflow(u16)`](Self::Overflow).
/// - **WebVTT** hours are unbounded (`u64`): uses [`NotPadded`](Self::NotPadded)
///   for non-digit input and [`HourOverflow`](Self::HourOverflow) when the
///   value exceeds `u64::MAX`.
/// - **ASS/SSA** hours are one or more digits and are *not* zero-padded
///   (`0:00:00.00`): uses [`NotPadded`](Self::NotPadded) for non-digit input
///   and [`HourOverflow`](Self::HourOverflow) when the value exceeds
///   `u64::MAX`.
#[derive(Debug, Clone, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap, thiserror::Error)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum ParseHourError {
  /// The hour component is not zero-padded to 2 digits (SRT),
  /// or contains non-digit characters (VTT).
  #[error("hour component is not zero-padded to 2 digits or contains invalid characters")]
  #[unwrap(ignore)]
  #[try_unwrap(ignore)]
  NotPadded,
  /// The hour component is out of the SRT range (0–999).
  #[error("hour component must be between 0-999, but was {0}")]
  Overflow(u16),
  /// The hour component overflowed `u64` (VTT unbounded hours).
  #[error("hour component overflowed")]
  #[unwrap(ignore)]
  #[try_unwrap(ignore)]
  HourOverflow,
  /// Not a valid number.
  #[error(transparent)]
  ParseInt(#[from] ParseIntError),
}

/// The error type for parsing millisecond components of timestamps.
#[derive(Debug, Clone, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap, thiserror::Error)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum ParseMillisecondError {
  /// The millisecond component is not zero-padded to 3 digits.
  #[error("millisecond component is not zero-padded to 3 digits")]
  #[unwrap(ignore)]
  #[try_unwrap(ignore)]
  NotPadded,
  /// The millisecond component is out of range (not between 0-999).
  #[error("millisecond component must be between 0-999, but was {0}")]
  Overflow(u16),
  /// Not a valid number.
  #[error(transparent)]
  ParseInt(#[from] ParseIntError),
}

/// The error type for parsing centisecond components of timestamps.
///
/// Centiseconds are the sub-second unit of ASS/SSA timestamps
/// (`H:MM:SS.cc`), where `cc` is always exactly two digits.
#[derive(Debug, Clone, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap, thiserror::Error)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum ParseCentisecondError {
  /// The centisecond component is not zero-padded to 2 digits.
  #[error("centisecond component is not zero-padded to 2 digits")]
  #[unwrap(ignore)]
  #[try_unwrap(ignore)]
  NotPadded,
  /// The centisecond component is out of range (not between 0-99).
  #[error("centisecond component must be between 0-99, but was {0}")]
  Overflow(u8),
  /// Not a valid number.
  #[error(transparent)]
  ParseInt(#[from] ParseIntError),
}

/// Specific reason why a timestamp has invalid structure.
///
/// This covers structural validation errors only (length, separators, digits).
/// Component range errors (hours, minutes, seconds, milliseconds,
/// centiseconds) are represented by their dedicated error types
/// ([`ParseHourError`], [`ParseMinuteError`], [`ParseSecondError`],
/// [`ParseMillisecondError`], [`ParseCentisecondError`]) which are separate
/// variants of [`crate::vtt::ParseVttError`] and
/// [`crate::ass::ParseAssError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TimestampError {
  /// The input is too short or has an invalid length for the timestamp form
  /// being parsed.
  #[error("invalid length")]
  InvalidLength,
  /// A separator (`.` or `:`) is not in the expected position.
  #[error("invalid format")]
  InvalidFormat,
  /// One or more digit positions contain non-digit bytes.
  #[error("invalid digits")]
  InvalidDigits,
}

/// The error type for parsing index numbers of subtitles.
#[derive(Debug, Clone, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap, thiserror::Error)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
pub enum ParseIndexNumberError {
  /// The index number is zero, which is invalid (must be between 1-18446744073709551615).
  #[error("index number cannot be zero")]
  Zero,
  /// The index number is out of range (not between 1-18446744073709551615).
  #[error("index number must be between 1-18446744073709551615")]
  Overflow,
  /// Not a valid index number.
  #[error(transparent)]
  ParseInt(#[from] ParseIntError),
}
