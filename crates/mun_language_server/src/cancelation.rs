use std::error::Error;

/// An error signifying a cancelled operation.
pub struct Canceled {
    // This is here so that you cannot construct a Canceled
    _private: (),
}

impl Canceled {
    fn new() -> Self {
        Canceled { _private: () }
    }

    pub fn throw() -> ! {
        // We use resume and not panic here to avoid running the panic
        // hook (that is, to avoid collecting and printing backtrace).
        std::panic::resume_unwind(Box::new(Canceled::new()))
    }
}

impl std::fmt::Display for Canceled {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("canceled")
    }
}

impl std::fmt::Debug for Canceled {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "Canceled")
    }
}

impl std::error::Error for Canceled {}

/// Returns true if the specified error is of type [`Canceled`]
pub(crate) fn is_canceled(e: &(dyn Error + 'static)) -> bool {
    e.downcast_ref::<Canceled>().is_some()
}
