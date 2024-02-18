//! Shared constants between serialization and deserialization logic.

pub const OPTION_SOME_VARIANT: u8 = u8::MAX;
pub const OPTION_NONE_VARIANT: u8 = u8::MIN;

// types are prefixed with labels so that their type can be
// inferred/asserted during deserialization.

pub const BYTES_BOOL: &'static [u8] = "bool".as_bytes();
pub const BYTES_BOOL_TRUE: &'static [u8] = "bool_t".as_bytes();
pub const BYTES_BOOL_FALSE: &'static [u8] = "bool_f".as_bytes();

pub const BYTES_UNIT: &'static [u8] = "unit".as_bytes();

pub const BYTES_STR: &'static [u8] = "str".as_bytes();

pub const BYTES_BYTES: &'static [u8] = "bytes".as_bytes();

pub const BYTES_OPTIONAL: &'static [u8] = "opt".as_bytes();
pub const BYTES_NONE: &'static [u8] = "opt_n".as_bytes();
pub const BYTES_SOME: &'static [u8] = "opt_s".as_bytes();

/// Prefix for numbers. All primitive numeric types are serialized as `u64` or `i64`,
/// into big endian.
pub const BYTES_NUM: &'static [u8] = "num".as_bytes();

/// Prefix for sequences like vectors
pub const BYTES_SEQ: &'static [u8] = "seq".as_bytes();
/// Prefix for sequences like maps
pub const BYTES_MAP: &'static [u8] = "map".as_bytes();

// byte delimiters for collections

// sequences with an arbitrary number of elements
pub const SEQ_OPEN: u8 = '[' as u8;
pub const SEQ_CLOSE: u8 = ']' as u8;

// sequences with a known number of elements
pub const SEQ_CONST_OPEN: u8 = '(' as u8;
pub const SEQ_CONST_CLOSE: u8 = ')' as u8;

// maps with an arbitrary number of elements
pub const MAP_OPEN: u8 = '{' as u8;
pub const MAP_CLOSE: u8 = '}' as u8;

pub const MAP_ENTRY_OPEN: u8 = '<' as u8;
pub const MAP_ENTRY_MID: u8 = '-' as u8;
pub const MAP_ENTRY_CLOSE: u8 = '>' as u8;
