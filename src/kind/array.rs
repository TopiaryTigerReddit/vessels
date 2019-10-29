use crate::{
    channel::{Channel, ForkHandle},
    ConstructResult, Kind,
};

use futures::{
    future::{join_all, ok, ready, try_join_all, BoxFuture, Ready},
    stream::once,
    FutureExt, SinkExt, StreamExt, TryFutureExt,
};

use std::{mem::MaybeUninit, ptr};

impl<T: Send + 'static> Kind for [T; 0] {
    type ConstructItem = ();
    type Error = ();
    type ConstructFuture = Ready<ConstructResult<Self>>;
    type DeconstructItem = ();
    type DeconstructFuture = Ready<()>;

    fn deconstruct<C: Channel<Self::DeconstructItem, Self::ConstructItem>>(
        self,
        _: C,
    ) -> Self::DeconstructFuture {
        ready(())
    }
    fn construct<C: Channel<Self::ConstructItem, Self::DeconstructItem>>(
        _: C,
    ) -> Self::ConstructFuture {
        ok([])
    }
}

macro_rules! array_impl {
    ($($len:expr => ($($n:tt $nn:ident)+))+) => {$(
        impl<T> Kind for [T; $len]
            where T: Kind
        {
            type ConstructItem = Vec<ForkHandle>;
            type Error = T::Error;
            type ConstructFuture = BoxFuture<'static, ConstructResult<Self>>;
            type DeconstructItem = ();
            type DeconstructFuture = BoxFuture<'static, ()>;
            fn deconstruct<C: Channel<Self::DeconstructItem, Self::ConstructItem>>(
                self,
                channel: C,
            ) -> Self::DeconstructFuture {
                let [$($nn),+] = self;
                Box::pin(
                    join_all(
                        vec![
                            $(channel.fork::<T>($nn)),+
                        ]
                    )
                    .then(move |handles| {
                        let channel = channel.sink_map_err(|_| panic!());
                        Box::pin(
                            once(ok(handles))
                                .forward(channel)
                                .unwrap_or_else(|_| panic!()),
                        )
                    }),
                )
            }
            fn construct<C: Channel<Self::ConstructItem, Self::DeconstructItem>>(
                channel: C,
            ) -> Self::ConstructFuture {
                Box::pin(
                    channel
                        .into_future()
                        .then(move |(item, channel)| {
                            try_join_all(
                                item.unwrap().into_iter().map(move |item| channel.get_fork::<T>(item))
                            ).map_ok(|items| {
                                let len = items.len();
                                if len != $len {
                                    panic!("expected data with {} elements, got {}", $len, len)
                                }
                                let mut arr = MaybeUninit::uninit();
                                for (i, item) in items.into_iter().enumerate() {
                                    unsafe { ptr::write((arr.as_mut_ptr() as *mut T).add(i), item) };
                                }
                                unsafe { arr.assume_init() }
                            })
                        })
                )
            }
        })+
    }
}

array_impl! {
    1 => (0 a)
    2 => (0 a 1 b)
    3 => (0 a 1 b 2 c)
    4 => (0 a 1 b 2 c 3 d)
    5 => (0 a 1 b 2 c 3 d 4 e)
    6 => (0 a 1 b 2 c 3 d 4 e 5 f)
    7 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g)
    8 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h)
    9 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8 i)
    10 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8 i 9 j)
    11 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8 i 9 j 10 k)
    12 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8 i 9 j 10 k 11 l)
    13 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8 i 9 j 10 k 11 l 12 m)
    14 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8 i 9 j 10 k 11 l 12 m 13 n)
    15 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8 i 9 j 10 k 11 l 12 m 13 n 14 o)
    16 => (0 a 1 b 2 c 3 d 4 e 5 f 6 g 7 h 8 i 9 j 10 k 11 l 12 m 13 n 14 o 15 p)
}
