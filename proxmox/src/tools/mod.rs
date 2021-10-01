//! This is a general utility crate used by all our rust projects.

use std::fmt;

use anyhow::*;
use lazy_static::lazy_static;

pub mod as_any;
pub mod borrow;
pub mod byte_buffer;
pub mod common_regex;
pub mod constnamedbitmap;
pub mod email;
pub mod fd;
pub mod fs;
pub mod io;
pub mod mmap;
pub mod parse;
pub mod serde;
pub mod time;
pub mod uuid;
pub mod vec;
pub mod systemd;

#[doc(inline)]
pub use uuid::Uuid;

#[doc(inline)]
pub use as_any::AsAny;

/// Evaluates to the offset (in bytes) of a given member within a struct
#[macro_export]
macro_rules! offsetof {
    ($ty:ty, $field:ident) => {
        unsafe { &(*(std::ptr::null::<$ty>())).$field as *const _ as usize }
    };
}

/// Statically assert the size of a type at compile time.
///
/// This should compile:
/// ```
/// # use proxmox::static_assert_size;
/// #[repr(C)]
/// struct Stuff {
///     value: [u8; 32]
/// }
/// static_assert_size!(Stuff, 32);
/// ```
///
/// This should fail to compile:
/// ```compile_fail
/// # use proxmox::static_assert_size;
/// #[repr(C)]
/// struct Stuff {
///     value: [u8; 32]
/// }
/// static_assert_size!(Stuff, 128);
/// ```
#[macro_export]
macro_rules! static_assert_size {
    ($ty:ty, $size:expr) => {
        const _: fn() -> () = || {
            let _ = ::std::mem::transmute::<[u8; $size], $ty>;
        };
    };
}

/// Macro to write error-handling blocks (like perl eval {})
///
/// #### Example:
/// ```
/// # use proxmox::try_block;
/// # use anyhow::*;
/// # let some_condition = false;
/// let result = try_block!({
///     if (some_condition) {
///         bail!("some error");
///     }
///     Ok(())
/// })
/// .map_err(|e| format_err!("my try block returned an error - {}", e));
/// ```

#[macro_export]
macro_rules! try_block {
    { $($token:tt)* } => {{ (|| -> Result<_,_> { $($token)* })() }}
}

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

/// Helper to provide a `Display` for arbitrary byte slices.
#[derive(Clone, Copy, Debug)]
pub struct AsHex<'a>(pub &'a [u8]);

impl AsHex<'_> {
    pub fn display_len(self) -> usize {
        self.0.len() * 2
    }

    pub fn to_hex_string(self) -> String {
        use std::fmt::Write;
        let mut s = String::with_capacity(self.display_len());
        write!(&mut s, "{}", self).expect("failed to format hex string");
        s
    }
}

impl fmt::Display for AsHex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8, 0u8];
        for b in self.0 {
            buf[0] = HEX_CHARS[(*b >> 4) as usize];
            buf[1] = HEX_CHARS[(*b & 0xf) as usize];
            f.write_str(unsafe { std::str::from_utf8_unchecked(&buf[..]) })?;
        }
        Ok(())
    }
}

pub fn digest_to_hex(digest: &[u8]) -> String {
    bin_to_hex(digest)
}

/// Convert a byte slice to a string of hexadecimal digits.
///
/// ```
/// # use proxmox::tools::bin_to_hex;
///
/// let text = bin_to_hex(&[1, 2, 0xff]);
/// assert_eq!(text, "0102ff");
/// ```
pub fn bin_to_hex(digest: &[u8]) -> String {
    AsHex(digest).to_string()
}

/// Convert an ascii character into a hex nibble.
fn hex_nibble_to_byte(b: u8) -> Result<u8, Error> {
    Ok(match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 0xA,
        b'A'..=b'F' => b - b'A' + 0xA,
        _ => bail!("not a hexadecimal digit: {}", char::from(b)),
    })
}

/// Parse hexadecimal digits into a byte array.
pub fn hex_to_bin_exact(hex: &str, out: &mut [u8]) -> Result<(), Error> {
    let bytes = hex.as_bytes();

    if bytes.len() != out.len() * 2 {
        bail!(
            "hexadecimal string has invalid length ({}, expected {})",
            bytes.len(),
            out.len() * 2,
        );
    }

    for i in 0..out.len() {
        let h = hex_nibble_to_byte(bytes[i * 2])?;
        let l = hex_nibble_to_byte(bytes[i * 2 + 1])?;
        out[i] = (h << 4) | l;
    }

    Ok(())
}

/// Convert a string of hexadecimal digits to a byte vector. Any non-digits are treated as an
/// error, so when there is possible whitespace in the string it must be stripped by the caller
/// first. Also, only full bytes are allowed, so the input must consist of an even number of
/// digits.
///
/// ```
/// # use proxmox::tools::hex_to_bin;
///
/// let data = hex_to_bin("aabb0123").unwrap();
/// assert_eq!(&data, &[0xaa, 0xbb, 0x01, 0x23]);
/// ```
pub fn hex_to_bin(hex: &str) -> Result<Vec<u8>, Error> {
    if (hex.len() % 2) != 0 {
        bail!("hex_to_bin: got wrong input length.");
    }

    let mut out = unsafe { vec::uninitialized(hex.len() / 2) };
    hex_to_bin_exact(hex, &mut out)?;
    Ok(out)
}

// FIXME: This should be renamed to contain the digest algorithm, so that the array's size makes
// sense.
pub fn hex_to_digest(hex: &str) -> Result<[u8; 32], Error> {
    let mut digest = [0u8; 32];
    hex_to_bin_exact(hex, &mut digest)?;
    Ok(digest)
}

#[test]
fn test_hex() {
    let mut out = [0u8; 5];
    hex_to_bin_exact("abCA01239f", &mut out).expect("failed to parse hex digit");
    assert_eq!(out, *b"\xab\xca\x01\x23\x9f");
    let v = hex_to_bin("abCA01239f").expect("failed to parse hex digit");
    assert_eq!(v, out);

    hex_to_bin_exact("abca01239", &mut out).expect_err("parsed invalid hex string");
    hex_to_bin_exact("abca01239fa", &mut out).expect_err("parsed invalid hex string");
    hex_to_bin_exact("abca0x239f", &mut out).expect_err("parsed invalid hex string");
}

/// Returns the hosts node name (UTS node name)
pub fn nodename() -> &'static str {
    lazy_static! {
        static ref NODENAME: String = {
            nix::sys::utsname::uname()
                .nodename()
                .split('.')
                .next()
                .unwrap()
                .to_owned()
        };
    }

    &NODENAME
}
