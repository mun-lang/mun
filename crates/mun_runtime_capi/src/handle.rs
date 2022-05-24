pub type Token = usize;

/// A type to uniquely index typed collections.
pub trait TypedHandle {
    /// Constructs a new `TypedHandle`.
    fn new(token: Token) -> Self;
    /// Retrieves the handle's token.
    fn token(&self) -> Token;
}

#[macro_export]
macro_rules! typed_handle {
    ($ty:ident) => {
        /// A C-style handle to an object.
        #[repr(C)]
        #[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
        pub struct $ty(crate::handle::Token);

        impl crate::handle::TypedHandle for $ty {
            fn new(token: crate::handle::Token) -> Self {
                Self(token)
            }

            fn token(&self) -> crate::handle::Token {
                self.0
            }
        }
    };
}
