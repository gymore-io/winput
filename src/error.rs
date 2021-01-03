use std::fmt;

/// Represents a Windows error.
#[derive(Clone, Debug)]
pub struct WindowsError {
    code: u32,
    description: Option<String>,
}

impl WindowsError {
    /// Creates a new [`WindowsError`] using the last error code retreived using the
    /// native `GetLastError` function.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::WindowsError;
    ///
    /// let error = WindowsError::from_last_error();
    /// println!("{:?}", &error.description);
    /// ```
    ///
    /// [`WindowsError`]: struct.WindowsError.html
    pub fn from_last_error() -> Self {
        use winapi::um::errhandlingapi::GetLastError;

        let last_error_code = unsafe { GetLastError() };
        Self::from_error_code(last_error_code)
    }

    /// Creates a new [`WindowsError`] using the given error code. The description of the
    /// error is retreived using the native `FormatMessageW` function.
    ///
    /// ## Example
    ///
    /// ```rust, ignore
    /// use winput::WindowsError;
    ///
    /// let error = WindowsError::from_error_code(101);
    /// println!("{:?}", &error.description);
    /// ```
    ///
    /// [`WindowsError`]: struct.WindowsError.html
    pub fn from_error_code(error_code: u32) -> Self {
        use std::{mem, ptr, slice};
        use winapi::um::winbase;

        let mut buffer_ptr: *mut u16 = ptr::null_mut();

        // Calling C code
        //
        // The function returns the number of `u16` characters that were written.
        let len = unsafe {
            winbase::FormatMessageW(
                winbase::FORMAT_MESSAGE_IGNORE_INSERTS
                    | winbase::FORMAT_MESSAGE_ALLOCATE_BUFFER
                    | winbase::FORMAT_MESSAGE_FROM_SYSTEM,
                ptr::null(),
                error_code,
                0,
                &mut buffer_ptr as *mut *mut u16 as _,
                0,
                ptr::null_mut(),
            )
        };

        if len == 0 {
            // The allocation failed / an invalid error code was provided
            return WindowsError {
                code: error_code,
                description: None,
            };
        }

        assert!(!buffer_ptr.is_null());
        assert_eq!(buffer_ptr as usize % mem::align_of::<u16>(), 0);

        // SAFETY:
        //  * `buffer_ptr` is non-null and well-aligned
        //  * `buffer_ptr` is not aliased anywere in the code
        //  * `buffer_ptr` is pointing to `len` properly initialized `u16`s.
        let slice = unsafe { slice::from_raw_parts_mut(buffer_ptr, len as _) };

        let description = String::from_utf16(slice).ok();

        // The message is now copied into the rust world.
        // We can free the allocated buffer.
        unsafe { winbase::LocalFree(buffer_ptr as _) };

        WindowsError {
            code: error_code,
            description,
        }
    }
}

impl fmt::Display for WindowsError {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref desc) = self.description {
            write!(f, "{} (code: {})", desc, self.code)
        } else {
            write!(f, "code: {}", self.code)
        }
    }
}
