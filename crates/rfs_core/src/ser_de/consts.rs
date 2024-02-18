//! Shared constants and definitions between serialization and deserialization logic.

/// The primitive data type that prefixes an arbitrary array of
/// bytes.
pub type ByteSizePrefix = u64;

pub const OPTION_SOME: u8 = u8::MAX;
pub const OPTION_NONE: u8 = u8::MIN;

pub const BOOL_TRUE: u8 = u8::MAX;
pub const BOOL_FALSE: u8 = u8::MIN;

// types are prefixed with labels so that their type can be
// inferred/asserted during deserialization.

/// `c` for condition
pub const PREFIX_BOOL: u8 = 'c' as u8;

/// `u` for unit
pub const PREFIX_UNIT: u8 = 'u' as u8;

/// `s` for string
pub const PREFIX_STR: u8 = 's' as u8;

/// `b` for bytes
pub const PREFIX_BYTES: u8 = 'b' as u8;

/// `o` for option
pub const PREFIX_OPTIONAL: u8 = 'o' as u8;

/// Prefix for numbers. All primitive numeric types are serialized as `u64` or `i64`,
/// into big endian.
///
/// `n` for numeric
pub const PREFIX_NUM: u8 = 'n' as u8;

/// `v` for vectors
pub const PREFIX_SEQ: u8 = 'v' as u8;

/// `t` for tuples
pub const PREFIX_SEQ_CONST: u8 = 't' as u8;

/// `m` for map
pub const PREFIX_MAP: u8 = 'm' as u8;

// byte delimiters for
// collections

// sequences with an arbitrary number of elements
pub const SEQ_OPEN: u8 = '[' as u8;
pub const SEQ_CLOSE: u8 = ']' as u8;

// sequences with a known number of elements
pub const SEQ_CONST_OPEN: u8 = '(' as u8;
pub const SEQ_CONST_CLOSE: u8 = ')' as u8;

// maps with an arbitrary number of elements
// or structs
pub const MAP_OPEN: u8 = '{' as u8;
pub const MAP_CLOSE: u8 = '}' as u8;

// these delimiters only apply to maps.
// the bytes in a serialized struct do not have any delimiters between
// each field.
pub const MAP_ENTRY_OPEN: u8 = '<' as u8;
pub const MAP_ENTRY_MID: u8 = '-' as u8;
pub const MAP_ENTRY_CLOSE: u8 = '>' as u8;
