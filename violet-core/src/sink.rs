// use futures::Stream;
// use futures_signals::signal::Mutable;

// struct Closed;

// /// A sink is a type which allows sending values.
// ///
// /// Values are sent synchronously.
// pub trait Sink<T> {
//     fn send(&self, value: T) -> Result<(), Closed>;
// }

// pub trait DuplexSink<T> {
//     type Sink: Sink<T>;
//     type Stream: Stream<Item = T>;

//     fn sink(&self) -> Self::Sink;
//     fn stream(&self) -> Self::Stream;
// }

// pub struct DuplexMutable<T, U> {
//     inner: Mutable<T>,

//     to_value: Box<dyn Fn(U) -> T>,
//     from_value: Box<dyn Fn(T) -> U>,
// }

// pub struct MappedSink<S, U, F> {
//     inner: S,
//     func: F,
// }

// impl<S, U, F> Sink<U> for MappedSink<S, U, F>
// where
//     S: Sink<U>,
//     F: Fn(U) -> U,
// {
//     fn send(&self, value: U) -> Result<(), Closed> {
//         self.inner.send((self.func)(value))
//     }
// }
