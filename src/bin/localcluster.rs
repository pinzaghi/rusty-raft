use rusty_raft::raft::RaftNode;

use prusti_contracts::*;

fn main() {

    let mut n0 = RaftNode::new(0);
    let mut n1 = RaftNode::new(1);
    let mut n2 = RaftNode::new(2);

    n0.timeout();

    assert!(n0.is_candidate());

    n0.request_vote();

}
