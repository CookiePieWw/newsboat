extern crate libc;

pub mod specifiers_iterator;
pub mod traits;

use std::ffi::{CStr, CString};
use std::vec::Vec;
use traits::*;

// TODO: write an internal module doc explaining the interplay between traits, fmt_arg, and fmt!

/// Helper function to `fmt!`. **Use it only through that macro!**
///
/// Returns a formatted string, or the size of the buffer that's necessary to hold the formatted
/// string.
#[doc(hidden)]
pub fn fmt_arg_with_buffer_size<T>(
    format_cstring: &CStr,
    arg_c_repr_holder: &T,
    buf_size: usize,
) -> Result<String, usize>
where
    T: CReprHolder,
{
    let mut buffer = Vec::<u8>::with_capacity(buf_size);
    // Filling the vector with ones because CString::new() doesn't want any zeroes in there. The
    // last byte is left unused, so that CString::new() can put a terminating zero byte there
    // without triggering a re-allocation.
    buffer.resize(buf_size - 1, 1);
    unsafe {
        // It's safe to use this function because we initialized the buffer and we know there are
        // no zeroes there
        let buffer = CString::from_vec_unchecked(buffer);
        let buffer_ptr = buffer.into_raw();
        let bytes_written = libc::snprintf(
            buffer_ptr,
            buf_size as libc::size_t,
            format_cstring.as_ptr() as *const libc::c_char,
            arg_c_repr_holder.to_c_repr(),
        ) as usize;
        let buffer = CString::from_raw(buffer_ptr);
        if bytes_written >= buf_size {
            Err(bytes_written + 1)
        } else {
            buffer.into_string().map_err(|_| 0)
        }
    }
}

/// Helper function to `fmt!`. **Use it only through that macro!**
#[doc(hidden)]
pub fn fmt_arg<T>(format: &str, arg: T) -> Option<String>
where
    T: Printfable,
{
    // Returns None if `format` contains a null byte
    CString::new(format).ok().and_then(|local_format_cstring| {
        // Returns None if a holder couldn't be obtained - e.g. the value is a String that
        // contains a null byte
        arg.to_c_repr_holder().and_then(|arg_c_repr_holder| {
            match fmt_arg_with_buffer_size(&local_format_cstring, &arg_c_repr_holder, 1024) {
                Ok(formatted) => Some(formatted),
                Err(buf_size) => {
                    fmt_arg_with_buffer_size(&local_format_cstring, &arg_c_repr_holder, buf_size)
                        .ok()
                }
            }
        })
    })
}

/// A safe-ish wrapper around `libc::snprintf`.
///
/// It pairs each format specifier ("%i", "%.2f" etc.) with a value, and passes those to
/// `libc::snprinf`; the results are then concatenated.
///
/// If a pair couldn't be formatted, it's omitted from the output. This can happen if:
/// - a format string contains null bytes;
/// - the string value to be formatted contains null bytes;
/// - `libc::snprintf` failed to format things even when given large enough buffer to do so.
#[macro_export]
macro_rules! fmt {
    ( $format:expr ) => {
        {
            let format: &str = $format;
            String::from(format)
        }
    };

    ( $format:expr, $( $arg:expr ),+ ) => {
        {
            let format: &str = $format;

            let mut result = String::new();

            use $crate::specifiers_iterator::SpecifiersIterator;
            let mut specifiers = SpecifiersIterator::from(format);

            $(
                let local_format = specifiers.next().unwrap_or("");
                if let Some(formatted_string) = $crate::fmt_arg(local_format, $arg) {
                    result.push_str(&formatted_string);
                }
            )+

            result
        }
    };
}

#[cfg(test)]
mod tests {
    extern crate libc;
    use std;

    #[test]
    fn returns_first_argument_if_it_is_the_only_one() {
        let input = String::from("Hello, world!");
        assert_eq!(fmt!(&input), input);
    }

    #[test]
    fn replaces_printf_format_with_text_representation_of_an_argument() {
        assert_eq!(fmt!("%i", 42), "42");
        assert_eq!(fmt!("%i", -13), "-13");
        assert_eq!(fmt!("%.3f", 3.1416), "3.142");
        assert_eq!(fmt!("%i %i", 100500, -191), "100500 -191");
    }

    #[test]
    fn formats_i32() {
        assert_eq!(fmt!("%i", 42i32), "42");
        assert_eq!(fmt!("%i", std::i32::MIN), "-2147483648");
        assert_eq!(fmt!("%i", std::i32::MAX), "2147483647");
    }

    #[test]
    fn formats_u32() {
        assert_eq!(fmt!("%u", 42u32), "42");
        assert_eq!(fmt!("%u", 0u32), "0");
        assert_eq!(fmt!("%u", std::u32::MAX), "4294967295");
    }

    #[test]
    fn formats_i64() {
        assert_eq!(fmt!("%li", 42i64), "42");
        assert_eq!(fmt!("%li", std::i32::MIN as i64 - 1), "-2147483649");
        assert_eq!(fmt!("%li", std::i32::MAX as i64 + 1), "2147483648");

        assert_eq!(fmt!("%lli", 42i64), "42");
        assert_eq!(fmt!("%lli", std::i64::MIN), "-9223372036854775808");
        assert_eq!(fmt!("%lli", std::i64::MAX), "9223372036854775807");
    }

    #[test]
    fn formats_u64() {
        assert_eq!(fmt!("%lu", 42u64), "42");
        assert_eq!(fmt!("%lu", 0u64), "0");
        assert_eq!(fmt!("%lu", std::u64::MAX), "18446744073709551615");

        assert_eq!(fmt!("%llu", 42u64), "42");
        assert_eq!(fmt!("%llu", 0u64), "0");
        assert_eq!(fmt!("%llu", std::u64::MAX), "18446744073709551615");
    }

    #[test]
    fn formats_void_ptr() {
        let x = 42i64;
        let ptr = &x as *const i64;
        assert_ne!(fmt!("%p", ptr as *const libc::c_void), "");
    }

    #[test]
    fn formats_null_ptr() {
        assert_eq!(fmt!("%p", std::ptr::null::<i32>()), "(nil)");
        assert_eq!(fmt!("%p", std::ptr::null::<u32>()), "(nil)");
        assert_eq!(fmt!("%p", std::ptr::null::<i64>()), "(nil)");
        assert_eq!(fmt!("%p", std::ptr::null::<u64>()), "(nil)");
        assert_eq!(fmt!("%p", std::ptr::null::<f32>()), "(nil)");
        assert_eq!(fmt!("%p", std::ptr::null::<f64>()), "(nil)");
    }

    #[test]
    fn formats_float() {
        let x = 42.0f32;
        assert_eq!(fmt!("%f", x), "42.000000");
        assert_eq!(fmt!("%.3f", x), "42.000");

        let y = 42e3f32;
        assert_eq!(fmt!("%e", y), "4.200000e+04");
        assert_eq!(fmt!("%.3e", y), "4.200e+04");

        assert_eq!(fmt!("%f", std::f32::INFINITY), "inf");
        assert_eq!(fmt!("%F", std::f32::INFINITY), "INF");

        assert_eq!(fmt!("%f", std::f32::NEG_INFINITY), "-inf");
        assert_eq!(fmt!("%F", std::f32::NEG_INFINITY), "-INF");

        assert_eq!(fmt!("%f", std::f32::NAN), "nan");
        assert_eq!(fmt!("%F", std::f32::NAN), "NAN");
    }

    #[test]
    fn formats_double() {
        let x = 42.0f64;
        assert_eq!(fmt!("%f", x), "42.000000");
        assert_eq!(fmt!("%.3f", x), "42.000");

        let y = 42e138f64;
        assert_eq!(fmt!("%e", y), "4.200000e+139");
        assert_eq!(fmt!("%.3e", y), "4.200e+139");

        assert_eq!(fmt!("%f", std::f64::INFINITY), "inf");
        assert_eq!(fmt!("%F", std::f64::INFINITY), "INF");

        assert_eq!(fmt!("%f", std::f64::NEG_INFINITY), "-inf");
        assert_eq!(fmt!("%F", std::f64::NEG_INFINITY), "-INF");

        assert_eq!(fmt!("%f", std::f64::NAN), "nan");
        assert_eq!(fmt!("%F", std::f64::NAN), "NAN");
    }

    #[test]
    fn formats_str_slice() {
        let input = "Hello, world!";
        assert_eq!(fmt!("%s", input), input);
    }

    #[test]
    fn formats_borrowed_string() {
        let input = String::from("Hello, world!");
        assert_eq!(fmt!("%s", &input), input);
    }

    #[test]
    fn formats_moved_string() {
        let input = String::from("Hello, world!");
        assert_eq!(fmt!("%s", input.clone()), input);
    }

    #[test]
    fn formats_2_megabyte_string() {
        let spacer = String::from(" ").repeat(1024 * 1024);
        let format = {
            let mut result = spacer.clone();
            result.push_str("%i");
            result.push_str(&spacer);
            result.push_str("%i");
            result
        };
        let expected = {
            let mut result = spacer.clone();
            result.push_str("42");
            result.push_str(&spacer);
            result.push_str("100500");
            result
        };
        assert_eq!(fmt!(&format, 42, 100500), expected);
    }
}
