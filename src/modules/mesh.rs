use esp_radio::{esp_now::{ EspNowReceiver, EspNowSender}};
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_radio::esp_now::BROADCAST_ADDRESS;
use heapless::Vec;
use crate::modules::{error::MeshError, node::{Node}, tree::Tree, message::{SendMessage, ReceiveMessage, MessageData, MessageType}};
#[allow(unused_imports)]
use esp_println::println;


const SEND_QUEUE_SIZE: usize = 16;
const RECEIVE_QUEUE_SIZE: usize = 16;
const RETURN_QUEUE_SIZE: usize = 16;



pub struct Mesh {
    send_queue: &'static Channel<NoopRawMutex, SendMessage, SEND_QUEUE_SIZE>,
    return_queue: &'static Channel<NoopRawMutex, MessageData, RETURN_QUEUE_SIZE>,
}

impl Mesh {

    pub fn new(spawner: Spawner, 
        send_queue: &'static Channel<NoopRawMutex, SendMessage, SEND_QUEUE_SIZE>,
        receive_queue: &'static Channel<NoopRawMutex, ReceiveMessage, RECEIVE_QUEUE_SIZE>,
        return_queue: &'static Channel<NoopRawMutex, MessageData, RETURN_QUEUE_SIZE>,
        receiver: EspNowReceiver<'static>,
        sender: EspNowSender<'static>,
    ) -> Self {
        let own_node = Node::new(BROADCAST_ADDRESS);
        let route = Tree::new(own_node);
        spawner.spawn(worker_task(receive_queue, send_queue, return_queue)).ok();
        spawner.spawn(receiver_task(receive_queue, receiver)).ok();
        spawner.spawn(sender_task(send_queue, sender, route)).ok();
        Mesh { send_queue, return_queue}
    }

    pub fn send(&self, data: &[u8], destination: Node) -> Result<(), MeshError> {
        let message = SendMessage::new(Vec::from_slice(data).map_err(|_|MeshError::NodeNotFound)?, destination, MessageType::Application);
        self.send_queue.try_send(message).map_err(|_|MeshError::NodeNotFound)
    }

    pub fn has_message(&self) -> bool {
        self.return_queue.len() > 0
    }

    pub fn get_message(&self) -> Result<MessageData, MeshError> {
        self.return_queue.try_receive().map_err(|_|MeshError::NodeNotFound)
    }

}

#[embassy_executor::task]
async fn worker_task(
    receive_queue: &'static Channel<NoopRawMutex, ReceiveMessage, SEND_QUEUE_SIZE>,
    send_queue: &'static Channel<NoopRawMutex, SendMessage, SEND_QUEUE_SIZE>,
    return_queue: &'static Channel<NoopRawMutex, MessageData, RETURN_QUEUE_SIZE>,
) {
    loop {
        let receive_message = receive_queue.receive().await;
        match (receive_message.message_type,
            receive_message.is_final_destination()
        ) {
            (MessageType::Application, true) => {return_queue.send(receive_message.data).await},
            (MessageType::Application, false) => {send_queue.send(receive_message.into()).await;
            },
        }
    }
}

#[embassy_executor::task]
async fn receiver_task(receive_queue: &'static Channel<NoopRawMutex, ReceiveMessage, SEND_QUEUE_SIZE>, mut receiver: EspNowReceiver<'static>) -> ! {
    loop {
        let r = receiver.receive_async().await;
        let mut data = MessageData::new();
        data.extend_from_slice(r.data()).unwrap();
        let destination = Node::new(r.info.src_address);
        let source = Node::new(r.info.src_address);
        receive_queue.send(ReceiveMessage::new(data, destination, source)).await;
    }
}

#[embassy_executor::task]
async fn sender_task(send_queue: &'static Channel<NoopRawMutex, SendMessage, SEND_QUEUE_SIZE>, mut sender: EspNowSender<'static>, route: Tree) -> ! {
    loop {
        let message = send_queue.receive().await;
        let next_hop = route.next_hop(message.final_destination).unwrap();
        sender.send_async(&next_hop.mac, &message.serialize().expect("could not serialize message")).await.unwrap();
    }
}

