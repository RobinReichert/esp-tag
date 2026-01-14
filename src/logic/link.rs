use crate::logic::{message::MessageData, node::Node, error::LinkError};
use core::future::Future;

#[cfg(feature = "hardware")]
pub type ActiveLink = crate::hardware::link::ESPNowLink;

#[cfg(not(feature = "hardware"))]
pub type ActiveLink = crate::logic::link::mock::MockLink;

#[derive(Debug)]
pub struct SendData {
    pub data: MessageData,
    pub destination: Node,
}

pub struct RecvData {
    pub data: MessageData,
    pub source: Node,
    pub destination: Node,
    pub rssi: i32,
}

pub trait Link<'a> {
    fn send(&'a self, data: MessageData, destination: Node) -> impl Future<Output = ()>;
    fn try_send(&self, data: MessageData, destination: Node) -> Result<(), LinkError>;
    fn receive(&'a self) -> impl Future<Output = RecvData>;
    fn try_receive(&self) -> Result<RecvData, LinkError>;
}


#[cfg(feature = "std")]
pub mod mock {
    use super::*;
    use std::collections::hash_map::HashMap;
    use tokio::sync::mpsc::{channel, Receiver, Sender};
    use tokio::sync::Mutex;

    pub struct MockLink {
        foreign_senders: HashMap<Node, Sender<MockMessage>>,
        receiver: Mutex<Receiver<MockMessage>>,
        sender: Sender<MockMessage>,
        node: Node,
    }

    struct MockMessage {
        data: MessageData,
        source: Node,
        destination: Node,
        rssi: i32,
    }

    impl MockLink {
        pub fn new(node: Node) -> Self {
            let foreign_senders: HashMap<Node, Sender<MockMessage>> = HashMap::new();
            let (sender, receiver) = channel(32);
            return MockLink { foreign_senders, receiver: Mutex::new(receiver), sender, node }
        }

        pub fn connect(&mut self, link: &MockLink) {
            self.foreign_senders.insert(link.node, link.sender.clone()); 
        }

    }

    impl<'a> Link<'a> for MockLink {
        fn send(&'a self, data: MessageData, destination: Node) -> impl Future<Output = ()> {
            async move {
                if let Some(sender) = self.foreign_senders.get(&destination) {
                    let message = MockMessage {
                        data,
                        source: self.node,
                        destination,
                        rssi: 255,
                    };
                    sender.send(message).await.unwrap();
                } else {
                    println!("not connected to {}", destination);
                }
            }
        }

        fn try_send(&self, data: MessageData, destination: Node) -> Result<(), LinkError> {
            if let Some(sender) = self.foreign_senders.get(&destination) {
                let message = MockMessage {
                    data,
                    source: self.node,
                    destination,
                    rssi: 255,
                };
                sender.try_send(message).map_err(|_| LinkError::MockError)?;
            } else {
                println!("not connected to {}", destination);
            }
            Ok(()) 
        }

        fn receive(&'a self) -> impl Future<Output = RecvData> {
            async {
                let message = self.receiver.lock().await.recv().await.unwrap();
                RecvData{rssi: message.rssi, data: message.data, source: message.source, destination: message.destination}
            }
        }

        fn try_receive(&self) -> Result<RecvData, LinkError> {
            let message = self.receiver.try_lock().map_err(|_| LinkError::MockError)?.try_recv().map_err(|_| LinkError::MockError)?;
            Ok(RecvData{rssi: message.rssi, data: message.data, source: message.source, destination: message.destination})
        }
    }
}
