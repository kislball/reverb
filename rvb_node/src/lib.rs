use log::debug;
use rvb_common::contract::{Contract, ContractCompiler};
use rvb_common::crypto::b64_encode;
use rvb_common::protocol::{Message, TransportMessage};
use rvb_common::transport::{Server, TransportError, TransportPeer};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, RwLock};
use tokio::task::{JoinHandle, yield_now};

#[derive(Debug)]
pub enum NodeError {
    TransportError(TransportError),
    SchemaError(rmp_serde::decode::Error),
    ProtocolError(rvb_common::protocol::ProtocolError),
    NoMessage,
}

#[derive(Debug, Clone, Copy)]
pub enum PeerInitStage {
    None,
    Hello,
    WhoAreYou,
    ItsMe,
    Welcome,
}

pub struct Peer {
    transport: Box<dyn TransportPeer>,
    stage: RwLock<PeerInitStage>,
    read_thread: Mutex<Option<JoinHandle<()>>>,
}

impl Peer {
    pub async fn next(&self) -> Result<TransportMessage, NodeError> {
        let raw = self
            .transport
            .recv()
            .await
            .map_err(NodeError::TransportError)?;
        let msg: TransportMessage = rmp_serde::from_slice(&raw).map_err(NodeError::SchemaError)?;
        Ok(msg)
    }

    pub async fn send(&self, msg: TransportMessage) -> Result<(), NodeError> {
        self.transport
            .send(rmp_serde::to_vec(&msg).unwrap())
            .await
            .map_err(NodeError::TransportError)
    }
}

pub struct NodeConfig {
    pub max_received_by: usize,
}

pub struct IncomingMessage {
    peer: Arc<Peer>,
    message: TransportMessage,
}

pub struct Node {
    pub identity: Vec<u8>,
    pub peers: RwLock<Vec<Arc<Peer>>>,
    pub config: NodeConfig,
    storage: sled::Db,
    contracts: HashMap<Vec<u8>, Arc<Mutex<Box<dyn Contract>>>>,
    contract_compiler: Box<dyn ContractCompiler>,
    server: Box<dyn Server>,
    msg_tx: Sender<IncomingMessage>,
    msg_rx: Mutex<Receiver<IncomingMessage>>,
    peer_tx: Sender<Box<dyn TransportPeer>>,
    peer_rx: Mutex<Receiver<Box<dyn TransportPeer>>>,
}

enum BroadcastStatus {
    Runtime,
    Network,
    Ok,
}

struct MessageContext {
    message: Message,
    peer: Arc<Peer>,
    transport: TransportMessage,
}

impl Node {
    #[must_use]
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }

    fn get_contract(&mut self, id: &[u8]) -> Option<Arc<Mutex<Box<dyn Contract>>>> {
        let contract = self.contracts.get(id);
        match contract {
            Some(x) => return Some(x.clone()),
            _ => {}
        };

        let contract_bytecode = self
            .storage
            .open_tree(b"contracts")
            .unwrap()
            .get(id)
            .ok()
            .flatten()?;

        let contract = self.contract_compiler
            .create_contract(contract_bytecode.as_ref())
            .ok()
            .map(|x| Arc::new(Mutex::new(x)))?;
        
        self.contracts.insert(id.to_vec(), contract.clone());
        
        Some(contract)
    }

    pub async fn receive_peers(&self) {
        let tx = self.peer_tx.clone();

        while let Ok(peer) = self.server.accept().await {
            if let Some(peer) = peer {
                if tx.send(peer).await.is_err() {
                    debug!("Failed to send peer to the node");
                }
            } else {
                debug!("Server returned None, no peer accepted");
            }
        }
    }

    pub async fn process(&self) {
        loop {
            self.process_next().await;
            yield_now().await;
        }
    }

    async fn process_next(&self) -> Result<(), NodeError> {
        while let Ok(peer) = self.peer_rx.lock().await.try_recv() {
            self.add_peer(peer).await;
        }

        yield_now().await;

        let msg = match self.msg_rx.lock().await.recv().await {
            Some(msg) => msg,
            None => return Err(NodeError::NoMessage),
        };

        let msgs: Vec<Message> = msg
            .message
            .clone()
            .try_into()
            .map_err(NodeError::ProtocolError)?;
        let msg = msgs
            .into_iter()
            .map(|x| MessageContext {
                message: x,
                peer: msg.peer.clone(),
                transport: msg.message.clone(),
            })
            .collect::<Vec<_>>();

        for msg in msg {
            if let Err(e) = self.process_message(msg).await {
                debug!("Failed to process message: {:?}", e);
            }
        }

        Ok(())
    }

    async fn process_message(&self, msg: MessageContext) -> Result<(), NodeError> {
        Ok(())
    }

    async fn add_peer(&self, peer: Box<dyn TransportPeer>) {
        let peer = Arc::new(Peer {
            transport: peer,
            stage: RwLock::new(PeerInitStage::None),
            read_thread: Mutex::new(None),
        });

        let mut read_thread_lock = peer.read_thread.lock().await;
        let cloned_peer = peer.clone();
        let tx = self.msg_tx.clone();

        *read_thread_lock = Some(tokio::task::spawn(async move {
            let peer = cloned_peer;

            while let Ok(msg) = peer.next().await {
                tx.send(IncomingMessage {
                    peer: peer.clone(),
                    message: msg,
                })
                .await
                .expect("The message channel must always be open");
                yield_now().await;
            }
        }));

        drop(read_thread_lock);

        self.peers.write().await.push(peer);
    }

    async fn broadcast(&self, msg: TransportMessage) {
        let peers = self.peers.read().await;
        let mut handles = Vec::with_capacity(peers.len());

        for peer in peers.as_slice() {
            let mut msg = msg.clone();

            if !msg.received_by.contains(&self.identity) {
                msg.received_by.push(self.identity.clone());
            }

            while msg.received_by.len() > self.config.max_received_by {
                msg.received_by.remove(0);
            }

            let peer = peer.clone();

            handles.push(tokio::spawn(async move { peer.send(msg).await }));
        }

        let res = futures::future::join_all(handles.into_iter()).await;
        let x = res.iter().map(|x| match x {
            Ok(e) => match e {
                Ok(()) => BroadcastStatus::Ok,
                _ => BroadcastStatus::Network,
            },
            Err(_) => BroadcastStatus::Runtime,
        });

        let (failed_runtime, failed_network, success) =
            x.fold((0, 0, 0), |(rt, net, ok), status| match status {
                BroadcastStatus::Runtime => (rt + 1, net, ok),
                BroadcastStatus::Network => (rt, net + 1, ok),
                BroadcastStatus::Ok => (rt, net, ok + 1),
            });

        if failed_runtime + failed_network > 0 {
            debug!(
                "Failed to broadcast message ID {} to {} peers ({} runtime, {} network). Success: {}",
                b64_encode(&msg.id),
                peers.len(),
                failed_runtime,
                failed_network,
                success
            );
        }
    }
}
