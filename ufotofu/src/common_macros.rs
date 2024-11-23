//! A family of macros for implementing non-ufofotu traits on invariant wrapper types.

/// Implement `Debug` for an opaque invariant wrapper type.
macro_rules! invarianted_impl_debug {
    ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)? ) => {
        impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
            core::fmt::Debug
        for $outer
            $(< $( $lt ),+ >)?
        {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
    }
}

/// Implement `AsRef` for an opaque invariant wrapper type.
macro_rules! invarianted_impl_as_ref {
    ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)?; $t:ty) => {
        impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
            core::convert::AsRef<$t>
        for $outer
            $(< $( $lt ),+ >)?
        {
            fn as_ref(&self) -> &$t {
                self.0.as_ref().as_ref()
            }
        }
    }
}

/// Implement `AsMut` for an opaque invariant wrapper type.
macro_rules! invarianted_impl_as_mut {
    ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)?; $t:ty) => {
        impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
            core::convert::AsMut<$t>
        for $outer
            $(< $( $lt ),+ >)?
        {
            fn as_mut(&mut self) -> &mut $t {
                self.0.as_mut().as_mut()
            }
        }
    }
}

/// Implement `Wrapper` for an opaque invariant wrapper type.
macro_rules! invarianted_impl_wrapper {
    ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)?; $t:ty) => {
        impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
            wrapper::Wrapper<$t>
        for $outer
            $(< $( $lt ),+ >)?
        {
            fn into_inner(self) -> $t {
                self.0.into_inner().into_inner()
            }
        }
    }
}
