use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
#[cfg(feature = "hardware")]
#[allow(unused_imports)]
use esp_println::println;
use embassy_sync::channel::Channel;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Duration, Timer};
use heapless::Vec;

use crate::logic::{error::{MeshError, TreeError}, link::{ActiveLink, Link}, message::{MessageContent, MessageData, MessageType, ReceiveMessage, SendMessage, BROADCAST_NODE}, node::Node, tree::Tree};

const RECV_QUEUE_SIZE: usize = 16;
const ORGANIZE_QUEUE_SIZE: usize = 16;
const MAX_NEWS: usize = 16;

pub struct Mesh {
    link: &'static ActiveLink,
    tree: &'static Tree,
    recv_queue: &'static Channel<NoopRawMutex, (MessageData, Node), RECV_QUEUE_SIZE>,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
}

impl Mesh {
    pub fn new(
        spawner: Spawner, 
        link: &'static ActiveLink, 
        tree: &'static Tree,
        recv_queue: &'static Channel<NoopRawMutex, (MessageData, Node), RECV_QUEUE_SIZE>,
        organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
    ) -> Result<Self, MeshError> {
        spawner.spawn(searcher_task(spawner, tree, link, organize_queue));
        spawner.spawn(dispatcher_task(link, tree, recv_queue, organize_queue)).map_err(|e| MeshError::SpawnError(e))?;
        Ok(Self {
            link,
            tree,
            recv_queue,
            organize_queue,
        })
    }

    pub async fn send(&self, data: MessageData, destination: Node) -> Result<(), MeshError> {
        let content = MessageContent::Application(data);
        Self::send_content(self.link, self.tree, content, destination).await

    }

    pub async fn receive(&self) -> (MessageData, Node) {
        self.recv_queue.receive().await
    }

    async fn send_content(link: &'static ActiveLink, tree: &'static Tree, content: MessageContent, destination: Node) -> Result<(), MeshError>{
        let msg = SendMessage::new(destination, content, None);
        Ok(link.send(msg.serialize().map_err(|e| MeshError::SerializationError(e))?, tree.next_hop(destination).map_err(|e| MeshError::TreeError(e))?).await)
    }
}

#[embassy_executor::task]
async fn searcher_task(
    spawner: Spawner,
    tree: &'static Tree,
    link: &'static ActiveLink, 
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    loop {
        let result: Result<Option<()>, MeshError> = (|| async {
            let send_msg = SendMessage::new(BROADCAST_NODE, MessageContent::Discovery, None);
            link.send(send_msg.serialize().map_err(|e| MeshError::SerializationError(e))?, BROADCAST_NODE).await;
            let got_invitation = async {                
                loop {
                    let recv_msg = organize_queue.receive().await;
                    match MessageType::from(&recv_msg.data) {
                        MessageType::Discovery => {
                            spawner.spawn(leader_task(spawner, link, organize_queue));
                            break;
                        },
                        MessageType::Invitation => {
                            spawner.spawn(follower_task(spawner, tree, link, organize_queue));
                            break;
                        },
                        _ => (),
                    }
                }
            };
            match select(
                Timer::after(Duration::from_secs(1)),
                got_invitation,
            ).await {
                Either::First(_) => Ok(None),
                Either::Second(_) => return Ok(Some(())),
            }
        })().await;
        match result {
            Err(e) => println!("{}", e),
            Ok(Some(_)) => break,
            Ok(None) => ()
        }
    }
}

#[embassy_executor::task]
async fn leader_task(
    spawner: Spawner,
    link: &'static ActiveLink, 
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let mut news: Vec<Node, MAX_NEWS> = Vec::new();
    loop{
        let msg = organize_queue.receive().await;
        match MessageType::from(&msg.data) {
            MessageType::Discovery => {
                match news.push(msg.final_source) {
                    Ok(_) => (),
                    Err(e) => println!("{}", e),
                } 
            },
            _ => (),
        }
    }
}

#[embassy_executor::task]
async  fn follower_task(
    spawner: Spawner,
    tree: &'static Tree,
    link: &'static ActiveLink, 
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let mut news: Vec<Node, MAX_NEWS> = Vec::new();
    loop {
        let msg = organize_queue.receive().await;
        match MessageType::from(&msg.data) {
            MessageType::Discovery => {
                match news.push(msg.final_source) {
                    Ok(_) => (),
                    Err(e) => println!("{}", e),
                } 
            },
            MessageType::RequestNews => {
                for new in news.iter() {
                    let content = MessageContent::SendNew(new.clone());
                    Mesh::send_content(link, tree, content, msg.final_source);
                }
                Mesh::send_content(link, tree, MessageContent::FinSendNew, msg.final_source);
            },
            _ => (),
        }
    }
}

#[embassy_executor::task]
async fn dispatcher_task(
    link: &'static ActiveLink, 
    tree: &'static Tree,
    recv_queue: &'static Channel<NoopRawMutex, (MessageData, Node), RECV_QUEUE_SIZE>,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) -> ! {
    loop {
        let data = link.receive().await;
        let result = (|| {
            let msg = ReceiveMessage::new(data.data, data.destination, data.source).map_err(|e| MeshError::ReceiveMessageError(e))?; 
            if !msg.is_final_destination() && !matches!(MessageType::from(&msg.data), MessageType::Discovery) {
                let send_msg: SendMessage = msg.into();
                link.try_send(
                    send_msg.serialize().map_err(|e| MeshError::SerializationError(e))?, 
                    tree.next_hop(send_msg.final_destination).map_err(|e| MeshError::TreeError(e))?
                ).map_err(|e| MeshError::LinkError(e))?;
                return Ok(());
            }
            if msg.is_organization() {
                organize_queue.try_send(msg).map_err(|e| MeshError::OrganizeQueueSendError(e))?;
                return Ok(());
            }
            match msg.data {
                MessageContent::Application(d) => recv_queue.try_send((d, msg.final_source)).map_err(|e| MeshError::ReceiveQueueSendError(e))?,
                _ => (),
            }
            Ok::<_, MeshError>(())
        })();
        if let Err(e) = result {
            println!("{}", e);
        }
    }
}
