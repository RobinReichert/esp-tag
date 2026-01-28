use embassy_executor::SpawnToken;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
pub use embassy_time::{Duration, Ticker};

use crate::logic::error::AsyncError;

pub type Spawner = embassy_executor::Spawner;

pub fn spawn<S>(spawner: &embassy_executor::Spawner, token: SpawnToken<S>) -> Result<(), AsyncError> {
    spawner.spawn(token).map_err(|_| AsyncError::SpawnError)
}

pub trait MyChannel<T> {
    type RecvFut<'a>: Future<Output = T> + 'a
    where
        Self: 'a;

    fn my_try_send(&self, v: T) -> Result<(), ()>;
    fn my_recv(&self) -> Self::RecvFut<'_>;
}

pub type Channel<T, const N: usize> = embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    T,
    N,
>;

impl<T, const N: usize> MyChannel<T> for Channel<T, N> {
    type RecvFut<'a>
        = embassy_sync::channel::ReceiveFuture<'a, CriticalSectionRawMutex, T, N>
    where
        T: 'a;

    fn my_try_send(&self, v: T) -> Result<(), ()> {
        self.try_send(v).map_err(|_| ())
    }

    fn my_recv(&self) -> Self::RecvFut<'_> {
        self.receive()
    }
}

pub type Mutex<T> = embassy_sync::mutex::Mutex<CriticalSectionRawMutex, T>;

pub enum Either<L, R> {
    First(L),
    Second(R),
}

pub async fn select<A, B>(a: A, b: B) -> Either<A::Output, B::Output>
where
    A: Future,
    B: Future,
{
    match embassy_futures::select::select(a, b).await {
        embassy_futures::select::Either::First(a) => Either::First(a),
        embassy_futures::select::Either::Second(b) => Either::Second(b),
    }
}

pub fn after(duration: Duration) -> impl Future<Output = ()> {
    embassy_time::Timer::after(duration)
}
