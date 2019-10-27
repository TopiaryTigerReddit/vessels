use crate::{
    channel::{Channel, ForkHandle},
    Kind,
};

use serde::{Deserialize, Serialize};

use futures::Future;

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug)]
pub enum KResult {
    Ok(ForkHandle),
    Err(ForkHandle),
}

impl<T, E> Kind for Result<T, E>
where
    T: Kind,
    E: Kind,
{
    type ConstructItem = KResult;
    type ConstructFuture = Box<dyn Future<Item = Self, Error = ()> + Send>;
    type DeconstructItem = ();
    type DeconstructFuture = Box<dyn Future<Item = (), Error = ()> + Send>;
    fn deconstruct<C: Channel<Self::DeconstructItem, Self::ConstructItem>>(
        self,
        channel: C,
    ) -> Self::DeconstructFuture {
        match self {
            Ok(v) => Box::new(
                channel
                    .fork(v)
                    .and_then(|h| channel.send(KResult::Ok(h)).then(|_| Ok(()))),
            ),
            Err(v) => Box::new(
                channel
                    .fork(v)
                    .and_then(|h| channel.send(KResult::Err(h)).then(|_| Ok(()))),
            ),
        }
    }
    fn construct<C: Channel<Self::ConstructItem, Self::DeconstructItem>>(
        channel: C,
    ) -> Self::ConstructFuture {
        Box::new(channel.into_future().then(|v| {
            match v {
                Ok(v) => match v.0.unwrap() {
                    KResult::Ok(r) => Box::new(v.1.get_fork::<T>(r).map(Ok).map_err(|_| panic!()))
                        as Box<dyn Future<Item = Result<T, E>, Error = ()> + Send>,
                    KResult::Err(r) => Box::new(
                        v.1.get_fork::<E>(r)
                            .then(|item| Ok(if let Ok(e) = item { Err(e) } else { panic!() })),
                    )
                        as Box<dyn Future<Item = Result<T, E>, Error = ()> + Send>,
                },
                _ => panic!("lol"),
            }
        }))
    }
}
