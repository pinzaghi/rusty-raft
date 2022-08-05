use std::future::Future;

use crate::raft::Message;
use crate::raft::NodeId;

pub trait RaftNetwork
{
    type FutureMessage: Future<Output = Message>;
    
    fn connect(&self, nid: NodeId);
    fn wait_for_message(&self) -> Self::FutureMessage;
    fn append_entries(&self);
   
}