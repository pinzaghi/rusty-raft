use std::rc::Rc;

use fxhash::{FxHashSet, FxHasher};

use rusty_raft::raft::{RaftNode, Message};

use prusti_contracts::*;

fn main() {

    let mut nodes = Vec::<Rc<RaftNode>>::new();

    let mut n0 = Rc::new(RaftNode::new(0));
    let mut n1 = Rc::new(RaftNode::new(1));
    let mut n2 = Rc::new(RaftNode::new(2));

    nodes.push(n0.clone());
    nodes.push(n1.clone());
    nodes.push(n2.clone());

    //n0.timeout();

    assert!(n0.is_candidate());

    let request_vote_msgs = n0.request_vote();

    //broadcast_messages(nodes, request_vote_msgs);

}

// fn broadcast_messages(nodes: Vec::<Rc<RaftNode>>, messages: FxHashSet::<Message>){
//     for n in nodes {
//         for m in &messages {
//             match m {
//                 Message::RequestVoteRequest{ term, last_log_index, last_log_term, source, dest} => { 
//                     if n.id == *dest { 
//                         n.receive_message(m.clone()) 
//                     } 
//                 },
                
//             }

//         }
//     }
// }
