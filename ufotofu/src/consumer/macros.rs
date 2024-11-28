//! Macros for quickly writing consumers.
//!
//! More specifically, a family of macros for wrapper consumers with an invariant wrapper and implementing various traits on the
//! resulting type by forwarding to the wrapper.
//!
//! Also, a family of macros to implement `local_nb::Consumer` by referring to an implementation of `sync::local_nb::Consumer`.
//!
//! See `common::consumer::into_slice` for example usage of all these macros.

// Macro syntax for handling generic parameters from https://stackoverflow.com/a/61189128

/// Create an opaque type of name `outer` that wraps the consumer `inner` with invariant checks.
macro_rules! invarianted_consumer_outer_type {
    ($(#[$doc:meta])* $outer:ident $inner:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)? ) => {
        $(#[$doc])*
        pub struct $outer $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?(ufotofu::common::consumer::Invariant<$inner $(< $( $lt ),+ >)?>);
    }
}

/// The method implementations of an opaque invariant wrapper around `Consumer`.
macro_rules! invarianted_consumer_methods {
    () => {
        async fn consume(&mut self, item: Self::Item) -> Result<(), Self::Error> {
            ufotofu::Consumer::consume(&mut self.0, item).await
        }

        async fn close(&mut self, fin: Self::Final) -> Result<(), Self::Error> {
            ufotofu::Consumer::close(&mut self.0, fin).await
        }

        async fn consume_full_slice<'b>(
            &mut self,
            buf: &'b [Self::Item],
        ) -> Result<(), ufotofu::ConsumeFullSliceError<Self::Error>>
        where
            Self::Item: Clone,
        {
            ufotofu::Consumer::consume_full_slice(&mut self.0, buf).await
        }
    };
}

/// Implement `Consumer` for an opaque invariant wrapper type generated by the invarianted_consumer_outer_type macro.
macro_rules! invarianted_impl_consumer {
    ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)? $(#[$doc_item:meta])? Item $t_item:ty; $(#[$doc_final:meta])? Final $t_final:ty; $(#[$doc_error:meta])? Error $t_error:ty) => {
        impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
            ufotofu::Consumer
        for $outer
            $(< $( $lt ),+ >)?
        {
            $(#[$doc_item])*
            type Item = $t_item;
            $(#[$doc_final])*
            type Final = $t_final;
            $(#[$doc_error])*
            type Error = $t_error;

            invarianted_consumer_methods!();
        }
    }
}

/// The method implementations of an opaque invariant wrapper around `BufferedConsmer`.
macro_rules! invarianted_buffered_consumer_methods {
    () => {
        async fn flush(&mut self) -> Result<(), Self::Error> {
            ufotofu::BufferedConsumer::flush(&mut self.0).await
        }
    };
}

/// Implement `BufferedConsumer` for an opaque invariant wrapper type generated by the invarianted_consumer_outer_type macro.
macro_rules! invarianted_impl_buffered_consumer {
    ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)?) => {
        impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
            ufotofu::BufferedConsumer
        for $outer
            $(< $( $lt ),+ >)?
        {
            invarianted_buffered_consumer_methods!();
        }
    }
}

/// The method implementations of an opaque invariant wrapper around `BulkConsumer`.
macro_rules! invarianted_bulk_consumer_methods {
    () => {
        async fn expose_slots<'kfhwkfwe>(
            &'kfhwkfwe mut self,
        ) -> Result<&'kfhwkfwe mut [Self::Item], Self::Error>
        where
            Self::Item: 'kfhwkfwe,
        {
            ufotofu::BulkConsumer::expose_slots(&mut self.0).await
        }

        async fn consume_slots(&mut self, amount: usize) -> Result<(), Self::Error> {
            ufotofu::BulkConsumer::consume_slots(&mut self.0, amount).await
        }

        async fn bulk_consume(&mut self, buf: &[Self::Item]) -> Result<usize, Self::Error> {
            ufotofu::BulkConsumer::bulk_consume(&mut self.0, buf).await
        }

        async fn bulk_consume_full_slice(
            &mut self,
            buf: &[Self::Item],
        ) -> Result<(), ufotofu::ConsumeFullSliceError<Self::Error>> {
            ufotofu::BulkConsumer::bulk_consume_full_slice(&mut self.0, buf).await
        }
    };
}

/// Implement `BulkConsumer` for an opaque invariant wrapper type generated by the invarianted_consumer_outer_type macro.
macro_rules! invarianted_impl_bulk_consumer {
    ($outer:ident $(< $( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+ >)?) => {
        impl $(< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
            ufotofu::BulkConsumer
        for $outer
            $(< $( $lt ),+ >)?
        {
            invarianted_bulk_consumer_methods!();
        }
    }
}
