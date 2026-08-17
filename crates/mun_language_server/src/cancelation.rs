use std::error::Error;

/// An error signifying a cancelled operation.
pub struct Canceled {
    // This is here so that you cannot construct a Canceled
    _private: (),
}

impl Canceled {
    pub(crate) fn new() -> Self {
        Canceled { _private: () }
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
    e.downcast_ref::<Canceled>().is_some() || e.downcast_ref::<ra_salsa::Cancelled>().is_some()
}
