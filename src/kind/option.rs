use crate::{
    channel::{Channel, ForkHandle},
    ConstructResult, DeconstructResult, Kind,
};

use futures::{future::BoxFuture, SinkExt, StreamExt};

use super::{ConstructError, DeconstructError};

impl<T> Kind for Option<T>
where
    T: Kind,
{
    type ConstructItem = Option<ForkHandle>;
    type ConstructError = ConstructError<T::ConstructError>;
    type ConstructFuture = BoxFuture<'static, ConstructResult<Self>>;
    type DeconstructItem = ();
    type DeconstructError = DeconstructError<T::DeconstructError>;
    type DeconstructFuture = BoxFuture<'static, DeconstructResult<Self>>;
    fn deconstruct<C: Channel<Self::DeconstructItem, Self::ConstructItem>>(
        self,
        mut channel: C,
    ) -> Self::DeconstructFuture {
        Box::pin(async move {
            channel
                .send(match self {
                    None => None,
                    Some(item) => Some(channel.fork(item).await?),
                })
                .await
                .map_err(From::from)
        })
    }
    fn construct<C: Channel<Self::ConstructItem, Self::DeconstructItem>>(
        mut channel: C,
    ) -> Self::ConstructFuture {
        Box::pin(async move {
            Ok(
                match channel.next().await.ok_or(ConstructError::Insufficient {
                    got: 0,
                    expected: 1,
                })? {
                    Some(item) => Some(channel.get_fork(item).await?),
                    None => None,
                },
            )
        })
    }
}
