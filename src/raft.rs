use std::option::Option;
use std::hash::Hash;
use std::hash::BuildHasher;
use std::collections::HashSet;

use fxhash::{FxHashSet, FxHasher};

use prusti_contracts::*;

pub type NodeId = u64;
pub type Term = u64;
pub type LogEntry = u64;

// A Raft node
pub struct RaftNode {
    id: NodeId,
    log: Vec<LogEntry>,
    state: State,
    current_term: Term,
    current_leader: Option<NodeId>,
    voted_for: Option<NodeId>,
    commit_index: u64,
    votes_responded: FxHashSet<NodeId>,
    votes_granted: FxHashSet<NodeId>,
    next_index: u64,
    match_index: u64
    // last_log_index: u64,
    // last_log_term: Term,
    
    // last_applied: u64,
    
}

#[extern_spec]
impl<T, S> HashSet<T, S>
{
    #[pure]
    pub fn len(&self) -> usize;

    #[ensures(self.len() == 0)]
    pub fn clear(&mut self);
}

#[extern_spec]
impl<T, S> HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    #[ensures(result ==> self.len() == old(self.len())+1)]
    #[ensures(!result ==> self.len() == old(self.len()))]
    pub fn insert(&mut self, value: T) -> bool;
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Follower,
    Candidate,
    Leader
}

pub enum Message {
    AppendEntries,
}

impl RaftNode{
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            log: Vec::new(),
            commit_index: 0,
            state: State::Follower,
            current_term: 0,
            current_leader: None,
            voted_for: None,
            votes_responded: FxHashSet::default(),
            votes_granted: FxHashSet::default(),
            next_index: 1,
            match_index: 0
            // last_log_index: 0,
            // last_log_term: 0
            // last_applied: 0,
        }
    }  

    #[requires(!self.is_leader())]
    pub fn timeout(&mut self){
        //println!("Node {0} timeout", self.id);
        match self.state {
            State::Follower => self.become_candidate(),
            State::Candidate => self.become_candidate(),
            _ => unreachable!()
        }
    }

    #[ensures(self.votes_granted.len() == 0)]
    #[ensures(old(self.current_term)+1 == self.current_term)]
    #[ensures(self.is_candidate())]
    fn become_candidate(&mut self){
        self.state = State::Candidate;
        self.current_term = self.current_term+1;
        self.voted_for = None;
        self.votes_responded = FxHashSet::default();
        self.votes_granted = FxHashSet::default();
        // Workaround for FxHashSet with prusti
        self.votes_responded.clear();
        self.votes_granted.clear();
        //println!("Size is {0}", self.votes_granted.len());
    }

    #[requires(self.is_candidate())]
    pub fn request_vote(&self) {

    }

    #[pure]
    pub fn is_candidate(&self) -> bool{
        matches!(self.state,State::Candidate)
    }

    #[pure]
    pub fn is_leader(&self) -> bool{
        matches!(self.state,State::Leader)
    }
}
