use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;

pub struct SharedBus<BUS> {
    bus: Mutex<NoopRawMutex, BUS>,
}

impl<BUS> SharedBus<BUS> {
    pub fn new(bus: BUS) -> Self {
        Self {
            bus: Mutex::new(bus),
        }
    }
}

pub struct SharedBusInterface<'a, I2C> {
    shared_bus: &'a SharedBus<I2C>,
}

impl<'a, I2C> SharedBusInterface<'a, I2C>
where
    I2C: embedded_hal_async::i2c::I2c,
{
    pub fn new(shared_bus: &'a SharedBus<I2C>) -> Self {
        Self { shared_bus }
    }
}

impl<I2C> embedded_hal_async::i2c::ErrorType for SharedBusInterface<'_, I2C>
where
    I2C: embedded_hal_async::i2c::I2c,
{
    type Error = <I2C as embedded_hal_async::i2c::ErrorType>::Error;
}

impl<I2C> embedded_hal_async::i2c::I2c for SharedBusInterface<'_, I2C>
where
    I2C: embedded_hal_async::i2c::I2c,
{
    fn read(
        &mut self,
        address: u8,
        read: &mut [u8],
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move { self.shared_bus.bus.lock().await.read(address, read).await }
    }

    fn write(
        &mut self,
        address: u8,
        write: &[u8],
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move { self.shared_bus.bus.lock().await.write(address, write).await }
    }

    fn write_read(
        &mut self,
        address: u8,
        write: &[u8],
        read: &mut [u8],
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            self.shared_bus
                .bus
                .lock()
                .await
                .write_read(address, write, read)
                .await
        }
    }

    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal_async::i2c::Operation<'_>],
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            self.shared_bus
                .bus
                .lock()
                .await
                .transaction(address, operations)
                .await
        }
    }
}
