#[macro_use]
extern crate erased_serde;
#[macro_use]
extern crate mopa;

pub mod channel;
pub use channel::OnTo;
use channel::{Channel, Target};
pub mod format;
pub mod kind;

pub use derive::kind;
use erased_serde::Serialize as ErasedSerialize;
use futures::{
    future::{ok, FutureResult},
    Future,
};
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ffi::{CString, OsString},
    marker::PhantomData,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU16, NonZeroU32,
        NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    time::{Duration, SystemTime},
};

lazy_static! {
    static ref REGISTRY: HashMap<TypeId, DeserializeFn> = {
        let mut map = HashMap::new();
        inventory::iter::<ErasedDeserialize>
            .into_iter()
            .for_each(|func| {
                if !map.contains_key(&func.ty) {
                    map.insert(func.ty, func.func);
                }
            });
        map
    };
}

pub(crate) struct ErasedDeserialize {
    ty: TypeId,
    func: DeserializeFn,
}

impl ErasedDeserialize {
    fn new(ty: TypeId, func: DeserializeFn) -> Self {
        ErasedDeserialize { ty, func }
    }
}

type DeserializeFn =
    fn(&mut dyn erased_serde::Deserializer) -> erased_serde::Result<Box<dyn SerdeAny>>;

inventory::collect!(ErasedDeserialize);

pub trait Kind: Sized + Send + 'static {
    type ConstructItem: Serialize + DeserializeOwned + Send + 'static;
    type ConstructFuture: Future<Item = Self> + Send + 'static;

    fn construct<C: Channel<Self::ConstructItem, Self::DeconstructItem>>(
        channel: C,
    ) -> Self::ConstructFuture;

    type DeconstructItem: Serialize + DeserializeOwned + Send + 'static;
    type DeconstructFuture: Future<Item = ()> + Send + 'static;

    fn deconstruct<C: Channel<Self::DeconstructItem, Self::ConstructItem>>(
        self,
        channel: C,
    ) -> Self::DeconstructFuture;

    #[doc(hidden)]
    const DO_NOT_IMPLEMENT_THIS_TRAIT_MANUALLY: ();
}

#[kind]
impl Kind for () {
    type ConstructItem = ();
    type DeconstructItem = ();
    type ConstructFuture = FutureResult<(), ()>;
    type DeconstructFuture = FutureResult<(), ()>;

    fn deconstruct<C: Channel<Self::DeconstructItem, Self::ConstructItem>>(
        self,
        _: C,
    ) -> Self::DeconstructFuture {
        ok(())
    }
    fn construct<C: Channel<Self::ConstructItem, Self::DeconstructItem>>(
        _: C,
    ) -> Self::ConstructFuture {
        ok(())
    }
}

#[kind]
impl<T: Send + 'static> Kind for PhantomData<T> {
    type ConstructItem = ();
    type ConstructFuture = FutureResult<PhantomData<T>, ()>;
    type DeconstructItem = ();
    type DeconstructFuture = FutureResult<(), ()>;

    fn deconstruct<C: Channel<Self::DeconstructItem, Self::ConstructItem>>(
        self,
        _: C,
    ) -> Self::DeconstructFuture {
        ok(())
    }
    fn construct<C: Channel<Self::ConstructItem, Self::DeconstructItem>>(
        _: C,
    ) -> Self::ConstructFuture {
        ok(PhantomData)
    }
}

macro_rules! primitive_impl {
    ($($ty:ident)+) => {$(
        #[kind]
        impl Kind for $ty {
            type ConstructItem = $ty;
            type ConstructFuture = Box<dyn Future<Item = $ty, Error = ()> + Send + 'static>;
            type DeconstructItem = ();
            type DeconstructFuture = Box<dyn Future<Item = (), Error = ()> + Send + 'static>;

            fn deconstruct<C: Channel<Self::DeconstructItem, Self::ConstructItem>>(
                self,
                channel: C,
            ) -> Self::DeconstructFuture {
                Box::new(channel.send(self).then(|_| Ok(())))
            }
            fn construct<C: Channel<Self::ConstructItem, Self::DeconstructItem>>(
                channel: C,
            ) -> Self::ConstructFuture
            {
                Box::new(
                    channel
                        .into_future()
                        .map_err(|e| panic!(e))
                        .map(|v| v.0.unwrap()),
                )
            }
        }
    )+};
}

primitive_impl!(bool isize i8 i16 i32 i64 usize u8 u16 u32 u64 f32 f64 char CString String Ipv4Addr SocketAddrV4 SocketAddrV6 SocketAddr SystemTime OsString Ipv6Addr Duration NonZeroU8 NonZeroU16 NonZeroU32 NonZeroU64 NonZeroUsize NonZeroI8 NonZeroI16 NonZeroI32 NonZeroI64 NonZeroIsize);

pub(crate) trait SerdeAny: erased_serde::Serialize + mopa::Any + Send {
    fn as_any(self) -> Box<dyn Any>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

mopafy!(SerdeAny);

serialize_trait_object!(SerdeAny);

impl<T: ?Sized> SerdeAny for T where T: ErasedSerialize + mopa::Any + Send {}
