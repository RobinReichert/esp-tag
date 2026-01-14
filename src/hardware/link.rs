use crate::logic::{error::LinkError, link::{Link, RecvData, SendData}, node::Node};
use crate::{logic::message::MessageData};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_radio::esp_now::{EspNowReceiver, EspNowSender};
use embassy_sync::channel::Channel;
use esp_println::println;

const SEND_QUEUE_SIZE: usize = 16;
const RECV_QUEUE_SIZE: usize = 16;

pub struct ESPNowLink {
    send_queue: &'static Channel<NoopRawMutex, SendData, SEND_QUEUE_SIZE>,
    recv_queue: &'static Channel<NoopRawMutex, RecvData, RECV_QUEUE_SIZE>,
}

impl ESPNowLink{
    pub fn new(
        spawner: Spawner,
        send_queue: &'static Channel<NoopRawMutex, SendData, SEND_QUEUE_SIZE>,
        recv_queue: &'static Channel<NoopRawMutex, RecvData, RECV_QUEUE_SIZE>,
        sender: EspNowSender<'static>,
        receiver: EspNowReceiver<'static>,
    ) -> Self {
        spawner.spawn(send_task(send_queue, sender));
        spawner.spawn(recv_task(recv_queue, receiver));
        ESPNowLink { send_queue, recv_queue }
    }
}

impl<'a> Link<'a> for ESPNowLink {
    fn send(&'a self, data: MessageData, destination: Node) -> impl Future<Output = ()> {
        async move {
            let send_data = SendData{ data, destination };
            self.send_queue.send(send_data).await
        }

    }

    fn try_send(&self, data: MessageData, destination: Node) -> Result<(), crate::logic::error::LinkError> {
        let send_data = SendData{ data, destination };
        self.send_queue.try_send(send_data).map_err(|e| LinkError::QueueFullError(e)) 
    }

     fn receive(&'a self) -> impl Future<Output = RecvData> {
        async  move {
            self.recv_queue.receive().await
        }
    }

    fn try_receive(&self) -> Result<RecvData, crate::logic::error::LinkError> {
        self.recv_queue.try_receive().map_err(|e| LinkError::QueueEmptyError(e))
    }
}

#[embassy_executor::task]
async fn send_task(
    send_queue: &'static Channel<NoopRawMutex, SendData, SEND_QUEUE_SIZE>,
    mut sender: EspNowSender<'static>,
) -> ! {
    loop {
        let data = send_queue.receive().await;
        if let Err(e) = sender.send_async(&data.destination.mac, &data.data).await {
            println!("Error while sending EspNow message:\n{}", e);
        }
    }
}

#[embassy_executor::task]
async fn recv_task(
    recv_queue: &'static Channel<NoopRawMutex, RecvData, RECV_QUEUE_SIZE>,
    mut receiver: EspNowReceiver<'static>,
) -> ! {
    loop {
        let received_data = receiver.receive_async().await;
        let mut data = MessageData::new();
        if let Err(e) = data.extend_from_slice(received_data.data()) {
            println!("Error while extending receive message:\n{}", e);
        }
        let source = Node::new(received_data.info.src_address);
        let destination = Node::new(received_data.info.dst_address);
        let rssi = received_data.info.rx_control.rssi;
        recv_queue.send(RecvData{data, source, destination, rssi}).await;
    }

}
