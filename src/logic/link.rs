use crate::logic::{error::LinkError, message::MessageData, node::Node};
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
    use crate::logic::message::BROADCAST_NODE;

    use super::*;
    use std::collections::hash_map::HashMap;
    use tokio::sync::Mutex;
    use tokio::sync::mpsc::{Receiver, Sender, channel};

    pub struct MockLink {
        foreign_senders: Mutex<HashMap<Node, Sender<MockMessage>>>,
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
            let foreign_senders: Mutex<HashMap<Node, Sender<MockMessage>>> =
                Mutex::new(HashMap::new());
            let (sender, receiver) = channel(32);
            return MockLink {
                foreign_senders,
                receiver: Mutex::new(receiver),
                sender,
                node,
            };
        }

        pub async fn connect(&self, link: &MockLink) {
            self.foreign_senders
                .lock()
                .await
                .insert(link.node, link.sender.clone());
        }
    }

    impl<'a> Link<'a> for MockLink {
        fn send(&'a self, data: MessageData, destination: Node) -> impl Future<Output = ()> {
            async move {
                let message = |destination| MockMessage {
                    data: data.clone(),
                    source: self.node,
                    destination,
                    rssi: 255,
                };
                if destination == BROADCAST_NODE {
                    for (node, sender) in self.foreign_senders.lock().await.iter() {
                        if let Err(e) = sender.send(message(*node)).await {
                            println!("failed to send broadcast to {}: {:?}", node, e);
                        }
                    }
                    return;
                }
                match self.foreign_senders.lock().await.get(&destination) {
                    Some(sender) => {
                        if let Err(e) = sender.send(message(destination)).await {
                            println!("failed to send to {}: {:?}", destination, e);
                        }
                    }
                    None => {
                        println!("not connected to {}", destination);
                    }
                }
            }
        }

        fn try_send(&self, data: MessageData, destination: Node) -> Result<(), LinkError> {
            let message = |destination| MockMessage {
                data: data.clone(),
                source: self.node,
                destination,
                rssi: 255,
            };
            if destination == BROADCAST_NODE {
                for (node, sender) in self
                    .foreign_senders
                    .try_lock()
                    .map_err(|_| LinkError::MockError)?
                    .iter()
                {
                    if let Err(e) = sender
                        .try_send(message(*node))
                        .map_err(|_| LinkError::MockError)
                    {
                        println!("failed to send broadcast to {}: {:?}", node, e);
                    }
                }
                return Ok(());
            }
            match self
                .foreign_senders
                .try_lock()
                .map_err(|_| LinkError::MockError)?
                .get(&destination)
            {
                Some(sender) => {
                    if let Err(e) = sender
                        .try_send(message(destination))
                        .map_err(|_| LinkError::MockError)
                    {
                        println!("failed to send to {}: {:?}", destination, e);
                    }
                }
                None => {
                    println!("not connected to {}", destination);
                }
            }
            Ok(())
        }

        fn receive(&'a self) -> impl Future<Output = RecvData> {
            async {
                let message = self.receiver.lock().await.recv().await.unwrap();
                RecvData {
                    rssi: message.rssi,
                    data: message.data,
                    source: message.source,
                    destination: message.destination,
                }
            }
        }

        fn try_receive(&self) -> Result<RecvData, LinkError> {
            let message = self
                .receiver
                .try_lock()
                .map_err(|_| LinkError::MockError)?
                .try_recv()
                .map_err(|_| LinkError::MockError)?;
            Ok(RecvData {
                rssi: message.rssi,
                data: message.data,
                source: message.source,
                destination: message.destination,
            })
        }
    }
}
