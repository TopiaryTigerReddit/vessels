mod array;
mod collections;
pub mod default;
mod functions;
mod future;
pub mod iterator;
mod option;
mod phantom_data;
mod primitives;
mod result;
pub mod serde;
mod sink;
mod stream;
mod tuple;
mod unit;
pub mod using;
mod wrapped;
pub use self::serde::Serde;
pub use default::Default;
pub use iterator::Iterator;

use failure::Fail;
use futures::{future::BoxFuture, stream::BoxStream, Sink as ISink};
use std::pin::Pin;

use crate::{channel::ChannelError, Kind};

pub type Stream<T> = BoxStream<'static, T>;
pub type Future<T> = BoxFuture<'static, T>;
pub type Sink<T, E> = Pin<Box<dyn ISink<T, Error = E> + Send>>;

pub trait AsKindMarker {}

#[derive(Fail, Debug)]
pub enum ConstructError<T: Fail> {
    #[fail(display = "{}", _0)]
    Construct(T),
    #[fail(display = "got {} items in construct, expected {}", got, expected)]
    Insufficient { got: usize, expected: usize },
}

#[derive(Fail, Debug)]
pub enum DeconstructError<T: Fail> {
    #[fail(display = "{}", _0)]
    Deconstruct(T),
    #[fail(display = "failed to send on underlying channel: {}", _0)]
    Send(ChannelError),
}

impl<T: Fail> From<T> for ConstructError<T> {
    fn from(input: T) -> Self {
        ConstructError::Construct(input)
    }
}

impl<T: Fail> From<T> for DeconstructError<T> {
    fn from(input: T) -> Self {
        DeconstructError::Deconstruct(input)
    }
}

impl<T: Fail> From<ChannelError> for DeconstructError<T> {
    fn from(input: ChannelError) -> Self {
        DeconstructError::Send(input)
    }
}

pub trait AsKind<M: AsKindMarker>: Sized {
    type Kind: Kind;

    fn into_kind(self) -> Self::Kind;
    fn from_kind(kind: Self::Kind) -> Self;
}
