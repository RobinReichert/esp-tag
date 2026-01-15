#![cfg(feature = "std")]
use tokio::sync::{futures, mpsc};
pub use std::time::Duration;

pub type Spawner = ();

pub fn spawn(_spawner: &(), fut: impl Future<Output = ()> + 'static) {
    tokio::task::spawn_local(fut);
}

pub type Channel<T, const N: usize> = StdChannel<T, N>;

pub struct StdChannel<T, const N: usize> {
    pub tx: mpsc::Sender<T>,
    pub rx: Mutex<mpsc::Receiver<T>>,
}

impl<T, const N: usize> StdChannel<T, N> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(N);
        Self { tx, rx: Mutex::new(rx) }
    }

    pub fn my_try_send(&self, v: T) -> Result<(), ()> {
        self.tx.try_send(v).map_err(|_| ())
    }

    pub async fn my_recv(&self) -> T {
        self.rx.lock().await.recv().await.expect("channel closed")
    }
}

pub type Mutex<T> = tokio::sync::Mutex<T>;

pub enum Either<L, R> {
    First(L),
    Second(R),
}

pub async fn select<A, B>(a: A, b: B) -> Either<A::Output, B::Output>
    where
    A: Future,
    B: Future,
{
    tokio::select! {
        v = a => Either::First(v),
        v = b => Either::Second(v),
    }
}

pub fn after(duration: Duration) -> impl Future<Output = ()> {
    tokio::time::sleep(duration)
}

pub struct Ticker {
    interval: tokio::time::Interval,
}

impl Ticker {
    pub fn every(duration: Duration) -> Self {
        Self {
            interval: tokio::time::interval(duration),
        }
    }

    pub async fn next(&mut self) {
        self.interval.tick().await;
    }
}

