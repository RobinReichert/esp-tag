use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Ticker, Timer};
#[cfg(feature = "hardware")]
#[allow(unused_imports)]
use esp_println::println;
use heapless::{LinearMap, Vec};

use crate::logic::{
    error::{MeshError, TreeError},
    link::{ActiveLink, Link},
    message::{
        BROADCAST_NODE, MessageContent, MessageData, MessageType, ReceiveMessage, SendMessage,
    },
    node::Node,
    tree::{self, Tree},
};

const RECV_QUEUE_SIZE: usize = 16;
const ORGANIZE_QUEUE_SIZE: usize = 16;
const MAX_NEWS: usize = 16;

pub struct Mesh {
    link: &'static ActiveLink,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    recv_queue: &'static Channel<NoopRawMutex, (MessageData, Node), RECV_QUEUE_SIZE>,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
}

impl Mesh {
    pub fn new(
        spawner: Spawner,
        link: &'static ActiveLink,
        tree: &'static Mutex<NoopRawMutex, Tree>,
        recv_queue: &'static Channel<NoopRawMutex, (MessageData, Node), RECV_QUEUE_SIZE>,
        organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
    ) -> Result<Self, MeshError> {
        spawner.spawn(searcher_task(spawner, tree, link, organize_queue));
        spawner
            .spawn(dispatcher_task(link, tree, recv_queue, organize_queue))
            .map_err(|e| MeshError::SpawnError(e))?;
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

    async fn send_content(
        link: &'static ActiveLink,
        tree: &'static Mutex<NoopRawMutex, Tree>,
        content: MessageContent,
        destination: Node,
    ) -> Result<(), MeshError> {
        let msg = SendMessage::new(destination, content, None);
        let next = tree
            .lock()
            .await
            .next_hop(destination)
            .map_err(|e| MeshError::TreeError(e))?;
        Ok(link
            .send(
                msg.serialize()
                    .map_err(|e| MeshError::SerializationError(e))?,
                next,
            )
            .await)
    }
}

#[embassy_executor::task]
async fn searcher_task(
    spawner: Spawner,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    loop {
        match run_search_round(spawner, tree, link, organize_queue).await {
            Ok(RoleDecision::Leader) => {
                spawner.spawn(leader_task(spawner, tree, link, organize_queue));
                break;
            }
            Ok(RoleDecision::Follower) => {
                spawner.spawn(follower_task(spawner, tree, link, organize_queue));
                break;
            }
            Ok(RoleDecision::Timeout) => {}
            Err(e) => println!("{}", e),
        }
    }
}

enum RoleDecision {
    Leader,
    Follower,
    Timeout,
}

async fn run_search_round(
    spawner: Spawner,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) -> Result<RoleDecision, MeshError> {
    send_discovery(link).await?;

    match select(
        Timer::after(Duration::from_secs(1)),
        wait_for_invitation(organize_queue, tree),
    )
    .await
    {
        Either::First(_) => Ok(RoleDecision::Timeout),
        Either::Second(role) => Ok(role),
    }
}

async fn send_discovery(link: &ActiveLink) -> Result<(), MeshError> {
    let msg = SendMessage::new(BROADCAST_NODE, MessageContent::Discovery, None);
    let data = msg.serialize().map_err(MeshError::SerializationError)?;
    link.send(data, BROADCAST_NODE).await;
    Ok(())
}

async fn wait_for_invitation(
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
    tree: &'static Mutex<NoopRawMutex, Tree>,
) -> RoleDecision {
    loop {
        let recv_msg = organize_queue.receive().await;
        match recv_msg.data {
            MessageContent::Discovery => return RoleDecision::Leader,
            MessageContent::UpsertEdge((n, p)) => {
                let parent = match p {
                    None => Some(recv_msg.final_source),
                    Some(node) if node == recv_msg.final_destination => None,
                    Some(node) => Some(node),
                };
                let new = match n {
                    None => recv_msg.final_source,
                    Some(node) => node,
                };
                tree.lock().await.upsert_edge(parent, new);
                return RoleDecision::Follower;
            }
            _ => {}
        }
    }
}

#[embassy_executor::task]
async fn leader_task(
    spawner: Spawner,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let mut news: Vec<(Node, i32), MAX_NEWS> = Vec::new();
    let mut ticker = Ticker::every(Duration::from_secs(5));

    loop {
        match select(organize_queue.receive(), ticker.next()).await {
            Either::First(msg) => handle_leader_message(&mut news, msg),
            Either::Second(_) => {
                process_news_round(&news, tree, link, organize_queue).await;
                news.clear();
            }
        }
    }
}

fn handle_leader_message(news: &mut Vec<(Node, i32), MAX_NEWS>, msg: ReceiveMessage) {
    if let MessageContent::Discovery = msg.data {
        if let Err((e, _)) = news.push((msg.final_source, msg.rssi)) {
            println!("{}", e);
        }
    }
}

async fn process_news_round(
    news: &Vec<(Node, i32), MAX_NEWS>,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let mut all_news = LinearMap::new();
    collect_local_news(news, &mut all_news);
    collect_remote_news(&mut all_news, tree, link, organize_queue).await;
    send_topology_updates(all_news, tree, link).await;
}

fn collect_local_news(
    news: &Vec<(Node, i32), MAX_NEWS>,
    all_news: &mut LinearMap<Node, (Option<Node>, i32), MAX_NEWS>,
) {
    for (node, rssi) in news {
        if let Err(e) = all_news.insert(*node, (None, *rssi)) {
            println!("{:?}", e);
        }
    }
}

async fn collect_remote_news(
    all_news: &mut LinearMap<Node, (Option<Node>, i32), MAX_NEWS>,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let nodes = {
        let t = tree.lock().await;
        t.into_iter().collect::<Vec<_, { tree::MAX_LEAFS }>>()
    };
    for (node, parent) in nodes {
        Mesh::send_content(link, tree, MessageContent::RequestNews, node).await;
        loop {
            match select(
                organize_queue.receive(),
                Timer::after(Duration::from_millis(500)),
            )
            .await
            {
                Either::First(response) => {
                    if !handle_news_response(all_news, node, response) {
                        break;
                    }
                }
                Either::Second(_) => break,
            }
        }
    }
}

fn handle_news_response(
    all_news: &mut LinearMap<Node, (Option<Node>, i32), MAX_NEWS>,
    parent: Node,
    response: ReceiveMessage,
) -> bool {
    match response.data {
        MessageContent::SendNew((node, rssi)) => {
            match all_news.get_mut(&node) {
                Some((best_parent, best_rssi)) if *best_rssi < rssi => {
                    *best_parent = Some(parent);
                    *best_rssi = rssi;
                }
                None => {
                    if let Err(e) = all_news.insert(node, (Some(parent), rssi)) {
                        println!("{:?}", e);
                    }
                }
                _ => {}
            }
            true
        }
        MessageContent::FinSendNew => false,
        _ => true,
    }
}

async fn send_topology_updates(
    all_news: LinearMap<Node, (Option<Node>, i32), MAX_NEWS>,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    link: &'static ActiveLink,
) {
    for (new_node, (parent, _)) in all_news {
        if let Err(e) = tree.lock().await.upsert_edge(None, new_node) {
            println!("{:?}", e);
            continue;
        }
        let nodes = {
            let t = tree.lock().await;
            t.into_iter().collect::<Vec<_, { tree::MAX_LEAFS }>>()
        };
        for (node, parent) in nodes {
            let content = MessageContent::UpsertEdge((Some(new_node), parent));
            Mesh::send_content(link, tree, content, node).await;
        }
        match parent {
            None => {
                send_initial_topology(new_node, tree, link);
            }
            Some(p) => {
                let content = MessageContent::RequestInitTopology(new_node);
                Mesh::send_content(link, tree, content, p).await;
            }
        }
    }
}

async fn send_initial_topology(
    new: Node,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    link: &'static ActiveLink,
) {
    let self_content = MessageContent::UpsertEdge((None, Some(new)));
    Mesh::send_content(link, tree, self_content, new).await;
    let nodes = {
        let t = tree.lock().await;
        t.into_iter().collect::<Vec<_, { tree::MAX_LEAFS }>>()
    };
    for (node, parent) in nodes {
        let foreign_content = MessageContent::UpsertEdge((Some(node), parent));
        Mesh::send_content(link, tree, foreign_content, new).await;
    }
}

#[embassy_executor::task]
async fn follower_task(
    spawner: Spawner,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let mut news: Vec<(Node, i32), MAX_NEWS> = Vec::new();
    loop {
        let msg = organize_queue.receive().await;
        match msg.data {
            MessageContent::Discovery => match news.push((msg.final_source, msg.rssi)) {
                Ok(_) => (),
                Err((n, r)) => println!("{}", n),
            },
            MessageContent::RequestNews => {
                for new in news.iter() {
                    let content = MessageContent::SendNew(new.clone());
                    Mesh::send_content(link, tree, content, msg.final_source).await;
                }
                Mesh::send_content(link, tree, MessageContent::FinSendNew, msg.final_source).await;
            }
            MessageContent::UpsertEdge((n, p)) => {
                let parent = match p {
                    None => Some(msg.final_source),
                    Some(node) if node == msg.final_destination => None,
                    Some(node) => Some(node),
                };
                let new = match n {
                    None => msg.final_source,
                    Some(node) => node,
                };
                tree.lock().await.upsert_edge(parent, new);
            }
            MessageContent::RequestInitTopology(n) => {
                send_initial_topology(n, tree, link).await;
            }
            _ => (),
        }
    }
}

#[embassy_executor::task]
async fn dispatcher_task(
    link: &'static ActiveLink,
    tree: &'static Mutex<NoopRawMutex, Tree>,
    recv_queue: &'static Channel<NoopRawMutex, (MessageData, Node), RECV_QUEUE_SIZE>,
    organize_queue: &'static Channel<NoopRawMutex, ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) -> ! {
    loop {
        let data = link.receive().await;
        let result = (|| async {
            let msg = ReceiveMessage::new(data.data, data.destination, data.source, data.rssi)
                .map_err(|e| MeshError::ReceiveMessageError(e))?;
            if !msg.is_final_destination()
                && !matches!(MessageType::from(&msg.data), MessageType::Discovery)
            {
                let send_msg: SendMessage = msg.into();
                let next = tree
                    .lock()
                    .await
                    .next_hop(send_msg.final_destination)
                    .map_err(|e| MeshError::TreeError(e))?;
                link.try_send(
                    send_msg
                        .serialize()
                        .map_err(|e| MeshError::SerializationError(e))?,
                    next,
                )
                .map_err(|e| MeshError::LinkError(e))?;
                return Ok(());
            }
            if msg.is_organization() {
                organize_queue
                    .try_send(msg)
                    .map_err(|e| MeshError::OrganizeQueueSendError(e))?;
                return Ok(());
            }
            match msg.data {
                MessageContent::Application(d) => recv_queue
                    .try_send((d, msg.final_source))
                    .map_err(|e| MeshError::ReceiveQueueSendError(e))?,
                _ => (),
            }
            Ok::<_, MeshError>(())
        })()
        .await;
        if let Err(e) = result {
            println!("{}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::{
        link::{ActiveLink, mock::MockLink},
        message,
    };

    fn setup_mesh(spawner: Spawner, link: ActiveLink) -> Mesh {
        let link = MockLink::new(Node::new([0, 0, 0, 0, 0, 1]));
        let tree = Mutex::new(Tree::new().unwrap());
        let recv_queue: Channel<NoopRawMutex, (MessageData, Node), 16> = Channel::new();
        let organize_queue: Channel<NoopRawMutex, message::ReceiveMessage, 16> = Channel::new();
        let mesh = Mesh::new(
            spawner,
            Box::leak(Box::new(link)),
            Box::leak(Box::new(tree)),
            Box::leak(Box::new(recv_queue)),
            Box::leak(Box::new(organize_queue)),
        )
        .unwrap();
        mesh
    }
}
