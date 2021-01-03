use std::fmt;

/// An error that can happen inside of this crate.
#[derive(Clone, Debug)]
pub enum Error {
    /// An error reported by Windows.
    OsError {
        /// The code of the error.
        code: u32,
        /// A message associated to the error code.
        message: String,
    },

    /// An `Input` could not be produced from a character.
    InvalidCharacter(char),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OsError { code, message } => write!(f, "code: {}: {}", code, message),
            Self::InvalidCharacter(c) => {
                write!(f, "{:?} cannot be turned into an `Input`", c)
            }
        }
    }
}

/// The result type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Produces an `Error::OsError` from the calling thread's last error code.
pub(crate) fn get_last_error() -> Error {
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winbase;
    use winapi::um::winbase::FormatMessageW;

    use std::{mem, ptr};

    // Calling C code
    //
    // This function returns the calling thread's last error code.
    let code = unsafe { GetLastError() };

    let message = unsafe {
        let mut buffer_ptr: *mut u16 = ptr::null_mut();

        // Calling C code
        //
        // This function will allocate a string at write a pointer to that string into
        // `buffer_ptr`.
        //
        // The allocated message is a null-terminated string with characters of 16 bits.
        let allocated_length = FormatMessageW(
            winbase::FORMAT_MESSAGE_ALLOCATE_BUFFER
                | winbase::FORMAT_MESSAGE_FROM_SYSTEM
                | winbase::FORMAT_MESSAGE_IGNORE_INSERTS,
            ptr::null(), // unused with `FORMAT_MESSAGE_FROM_SYSTEM`,
            code,
            0, // let the system pick a language
            &mut buffer_ptr as *mut *mut u16 as *mut u16,
            0, // minimum size for the allocated buffer
            ptr::null_mut(),
        );

        if allocated_length == 0 {
            // The function failed.
            String::from("Failed to get a message for this error code")
        } else {
            // The function succeeded.
            assert!(!buffer_ptr.is_null());
            assert_eq!(buffer_ptr as usize % mem::align_of::<u16>(), 0);

            // SAFETY:
            //  * The pointer is non null
            //  * `FormatMessageW` allocated `allocated_length` characters
            //  * We are the only owner of the data
            //  * The pointer is alligned
            //
            // The null character is not included in the slice.
            let slice = std::slice::from_raw_parts_mut(buffer_ptr, allocated_length as _);

            let message = String::from_utf16_lossy(slice);

            // The message was copied, we can deallocate it.
            winbase::LocalFree(buffer_ptr as _);

            message
        }
    };

    Error::OsError { code, message }
}
