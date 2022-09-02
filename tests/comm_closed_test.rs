use fxhash::FxHashSet;
use std::env;
use rusty_raft::raft::{RaftNode, NodeId};

// We break Communication Closure by making a process receive and AppendEntriesRequest before any election
#[test]
fn node_is_elected() {

    env::set_var("RUST_BACKTRACE", "1");

    let mut n0 = RaftNode::new(0);
    let mut n1 = RaftNode::new(1);
    let mut n2 = RaftNode::new(2);

    let mut config = FxHashSet::<NodeId>::default();
    config.insert(n0.id());
    config.insert(n1.id());
    config.insert(n2.id());
    
    n0.init(&config);
    n1.init(&config);
    n2.init(&config);

    // n0 timeouts and wants to become leader
    let msg_request_vote = n0.timeout();
    assert!(n0.is_candidate());

    let msg_request_vote_response0 = n0.receive_message(msg_request_vote.clone());
    let msg_request_vote_response1 = n1.receive_message(msg_request_vote.clone());
    // Message to N2 is lost
    //let msg_request_vote_response2 = n2.receive_message(msg_request_vote.clone());

    n0.receive_message(msg_request_vote_response0.unwrap());
    n0.receive_message(msg_request_vote_response1.unwrap());
    //n0.receive_message(msg_request_vote_response2.unwrap());

    assert!(n0.is_leader());
    assert!(n1.is_follower());
    assert!(n2.is_follower());

    assert!(n0.current_term() == 1);
    assert!(n1.current_term() == 1);
    //assert!(n2.current_term() == 1);

    // Client request
    n0.client_request(42);
    let msg_append1 = n0.append_entries(&n1.id());
    let msg_append2 = n0.append_entries(&n2.id());
    n1.receive_message(msg_append1);
    n2.receive_message(msg_append2);

    // Heartbeat 1, followers reply accepting previous entry
    let msg_append1 = n0.append_entries(&n1.id());
    let msg_append2 = n0.append_entries(&n2.id());
    let msg_append_response1 = n1.receive_message(msg_append1).unwrap();
    let msg_append_response2 = n2.receive_message(msg_append2).unwrap();
    n0.receive_message(msg_append_response1);
    n0.receive_message(msg_append_response2);

    // At this point Leader has a quorum of acks, it can commit
    assert!(n0.commit_index() == 1);

    // Heartbeat 2, followers commit
    let msg_append1 = n0.append_entries(&n1.id());
    let msg_append2 = n0.append_entries(&n2.id());
    n1.receive_message(msg_append1);
    n2.receive_message(msg_append2);

    assert!(n1.commit_index() == 1);
    assert!(n2.commit_index() == 1);

    // N0, N1 and N2 have the same log
    assert!(n0.log().len() == n1.log().len() && n1.log().len() == n2.log().len());
    assert!(n0.log().lookup(1) == n1.log().lookup(1) && n1.log().lookup(1) == n2.log().lookup(1) && n1.log().lookup(1).value == 42);

}