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
    pub destination:Node,
}

pub trait Link<'a> {
    fn send(&'a self, data: MessageData, destination: Node) -> impl Future<Output = ()>;
    fn try_send(&self, data: MessageData, destination: Node) -> Result<(), LinkError>;
    fn receive(&'a self) -> impl Future<Output = RecvData>;
    fn try_receive(&self) -> Result<RecvData, LinkError>;
}


mod mock {
    use super::*;
    pub struct MockLink {
    }

    impl<'a> Link<'a> for MockLink {
        fn send(&'a self, data: MessageData, destination: Node) -> impl Future<Output = ()> {
           async {
            }
        }

        fn try_send(&self, data: MessageData, destination: Node) -> Result<(), LinkError> {
           Ok(()) 
        }

        fn receive(&'a self) -> impl Future<Output = RecvData> {
            async {
                RecvData{data: MessageData::new(), source: Node::new([0, 0, 0, 0, 0, 0]), destination: Node::new([1,2, 3, 4, 5, 6])}
            }
        }

        fn try_receive(&self) -> Result<RecvData, LinkError> {
           Err(LinkError::MockError) 
        }
    }
}
