#[cfg(feature = "hardware")]
#[allow(unused_imports)]
use esp_println::println;

#[cfg(feature = "hardware")]
use crate::hardware::asynchronous::{self, MyChannel};

#[cfg(feature = "std")]
use crate::logic::asynchronous;

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
    tree: &'static asynchronous::Mutex<Tree>,
    recv_queue: &'static asynchronous::Channel<(MessageData, Node), RECV_QUEUE_SIZE>,
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
}

impl Mesh {
    pub fn new(
        spawner: asynchronous::Spawner,
        link: &'static ActiveLink,
        tree: &'static asynchronous::Mutex<Tree>,
        recv_queue: &'static asynchronous::Channel<(MessageData, Node), RECV_QUEUE_SIZE>,
        organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
    ) -> Result<Self, MeshError> {
        asynchronous::spawn(&spawner, searcher_task(spawner, tree, link, organize_queue));
        asynchronous::spawn(&spawner, dispatcher_task(link, tree, recv_queue, organize_queue));
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
        self.recv_queue.my_recv().await
    }

    async fn send_content(
        link: &'static ActiveLink,
        tree: &'static asynchronous::Mutex<Tree>,
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

#[cfg_attr(feature = "hardware", embassy_executor::task)]
async fn searcher_task(
    spawner: asynchronous::Spawner,
    tree: &'static asynchronous::Mutex<Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    loop {
        match run_search_round(spawner, tree, link, organize_queue).await {
            Ok(RoleDecision::Leader) => {
                println!("leader");
                asynchronous::spawn(&spawner, leader_task(spawner, tree, link, organize_queue));
                break;
            }
            Ok(RoleDecision::Follower) => {
                println!("follower");
                asynchronous::spawn(&spawner, follower_task(spawner, tree, link, organize_queue));
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
    spawner: asynchronous::Spawner,
    tree: &'static asynchronous::Mutex<Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) -> Result<RoleDecision, MeshError> {
    match asynchronous::select(
        asynchronous::after(asynchronous::Duration::from_secs(1)),
        wait_for_invitation(organize_queue, tree),
    )
    .await
    {
        asynchronous::Either::First(_) => {
            send_discovery(link).await?;
            Ok(RoleDecision::Timeout)
        },
        asynchronous::Either::Second(role) => Ok(role),
    }
}

async fn send_discovery(link: &ActiveLink) -> Result<(), MeshError> {
    let msg = SendMessage::new(BROADCAST_NODE, MessageContent::Discovery, None);
    let data = msg.serialize().map_err(MeshError::SerializationError)?;
    link.send(data, BROADCAST_NODE).await;
    Ok(())
}

async fn wait_for_invitation(
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
    tree: &'static asynchronous::Mutex<Tree>,
) -> RoleDecision {
    loop {
        let recv_msg = organize_queue.my_recv().await;
        match recv_msg.data {
            MessageContent::Discovery => {
                return RoleDecision::Leader;
            },
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
                match tree.lock().await.upsert_edge(parent, new) {
                    Err(e) => println!("{}", e),
                    _ => (),
                }
                print!("{}", tree.lock().await);
                return RoleDecision::Follower;
            }
            _ => {}
        }
    }
}

#[cfg_attr(feature = "hardware", embassy_executor::task)]
async fn leader_task(
    spawner: asynchronous::Spawner,
    tree: &'static asynchronous::Mutex<Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let mut news: Vec<(Node, i32), MAX_NEWS> = Vec::new();
    let mut ticker = asynchronous::Ticker::every(asynchronous::Duration::from_secs(3));

    loop {
        match asynchronous::select(organize_queue.my_recv(), ticker.next()).await {
            asynchronous::Either::First(msg) => handle_leader_message(&mut news, msg),
            asynchronous::Either::Second(_) => {
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
    tree: &'static asynchronous::Mutex<Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
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
    tree: &'static asynchronous::Mutex<Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let nodes = {
        let t = tree.lock().await;
        t.into_iter().collect::<Vec<_, { tree::MAX_LEAFS }>>()
    };
    for (node, parent) in nodes {
        Mesh::send_content(link, tree, MessageContent::RequestNews, node).await;
        loop {
            match asynchronous::select(
                organize_queue.my_recv(),
                asynchronous::after(asynchronous::Duration::from_millis(500)),
            )
            .await
            {
                asynchronous::Either::First(response) => {
                    if !handle_news_response(all_news, node, response) {
                        break;
                    }
                }
                asynchronous::Either::Second(_) => break,
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
    tree: &'static asynchronous::Mutex<Tree>,
    link: &'static ActiveLink,
) {
    for (new_node, (parent, _)) in all_news {
        let nodes = {
            let t = tree.lock().await;
            t.into_iter().collect::<Vec<_, { tree::MAX_LEAFS }>>()
        };
        for (node, parent) in nodes {
            let content = MessageContent::UpsertEdge((Some(new_node), parent));
            Mesh::send_content(link, tree, content, node).await;
        }
        if let Err(e) = tree.lock().await.upsert_edge(None, new_node) {
            println!("{:?}", e);
            continue;
        }
        print!("{}", tree.lock().await);
        match parent {
            None => {
                send_initial_topology(new_node, tree, link).await;
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
    tree: &'static asynchronous::Mutex<Tree>,
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

#[cfg_attr(feature = "hardware", embassy_executor::task)]
async fn follower_task(
    spawner: asynchronous::Spawner,
    tree: &'static asynchronous::Mutex<Tree>,
    link: &'static ActiveLink,
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
    let mut news: Vec<(Node, i32), MAX_NEWS> = Vec::new();
    loop {
        let msg = organize_queue.my_recv().await;
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

#[cfg_attr(feature = "hardware", embassy_executor::task)]
async fn dispatcher_task(
    link: &'static ActiveLink,
    tree: &'static asynchronous::Mutex<Tree>,
    recv_queue: &'static asynchronous::Channel<(MessageData, Node), RECV_QUEUE_SIZE>,
    organize_queue: &'static asynchronous::Channel<ReceiveMessage, ORGANIZE_QUEUE_SIZE>,
) {
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
                    .my_try_send(msg)
                    .map_err(|e| MeshError::OrganizeQueueSendError())?;
                return Ok(());
            }
            match msg.data {
                MessageContent::Application(d) => recv_queue
                    .my_try_send((d, msg.final_source))
                    .map_err(|e| MeshError::ReceiveQueueSendError())?,
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
    use core::time::Duration;

    use super::*;
    use tokio::{task::LocalSet, time::sleep};
    use crate::logic::{
        link::{ActiveLink, mock::MockLink},
        message,
    };

    fn setup_mesh(spawner: asynchronous::Spawner, link: ActiveLink) -> Mesh {
        let tree = asynchronous::Mutex::new(Tree::new().unwrap());
        let recv_queue: asynchronous::Channel<(MessageData, Node), 16> = asynchronous::Channel::new();
        let organize_queue: asynchronous::Channel<message::ReceiveMessage, 16> = asynchronous::Channel::new();
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

    #[tokio::test(flavor = "current_thread")]
    async fn mesh_creation_test() {
        let local = LocalSet::new();
        let link = MockLink::new(Node::new([0, 0, 0, 0, 0, 1]));
        local.run_until(async {
            let _mesh = setup_mesh((), link);
        }).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mesh_send_to_self_returns_error() {
        let local = LocalSet::new();
        let self_node = Node::new([0, 0, 0, 0, 0, 1]);
        let link = MockLink::new(self_node);
        local
            .run_until(async {
                let mesh = setup_mesh((), link);
                let payload = MessageData::from([1, 2, 3]);
                let result = mesh.send(payload, self_node).await;
                assert!(result.is_err(), "sending to self must error");
            })
        .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mesh_send_receive_between_two_nodes() {
        let local = LocalSet::new();

        let a = Node::new([0, 0, 0, 0, 0, 1]);
        let b = Node::new([0, 0, 0, 0, 0, 2]);
        let mut link_a = MockLink::new(a);
        let mut link_b = MockLink::new(b);
        link_a.connect(&link_b);
        link_b.connect(&link_a);

        local
            .run_until(async {
                let mesh_a = setup_mesh((), link_a);
                sleep(Duration::from_millis(500)).await;
                let mesh_b = setup_mesh((), link_b);

                sleep(Duration::from_secs(5)).await;

                let payload = MessageData::from([42]);
                mesh_a.send(payload.clone(), b).await.unwrap();

                sleep(Duration::from_secs(1)).await;

            let (recv, src) = mesh_b.receive().await;
            assert_eq!(recv, payload);
            assert_eq!(src, a);
        })
        .await;
}


}
