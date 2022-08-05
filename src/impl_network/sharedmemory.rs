use std::collections::HashMap;
use std::future;

use crate::network::{RaftNetwork};
use crate::raft::NodeId;
use crate::raft::Message;

pub struct SharedMemoryNetwork {
    connections: HashMap<NodeId, Connection>
}

pub struct Connection {

}

impl SharedMemoryNetwork {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new()
        }
    }
}

impl RaftNetwork for SharedMemoryNetwork {
    type FutureMessage = future::Ready<Message>;
    
    fn connect(&self, nid: NodeId){

    }
    
    fn append_entries(&self){

    }
    
    fn wait_for_message(&self) -> Self::FutureMessage{

        future::ready(Message::AppendEntries)
    }
}