use std::option::Option;
use std::hash::Hash;
use std::hash::BuildHasher;
use std::collections::HashSet;
use std::cmp;

use fxhash::{FxHashSet, FxHashMap, FxHasher};

use crate::log::{Log, LogEntry};

use prusti_contracts::*;

pub type NodeId = usize;
pub type Term = usize;
pub type LogIndex = usize;
pub type Config = FxHashSet<NodeId>;

// A Raft node
pub struct RaftNode {
    pub id: NodeId,
    log: Log,
    state: State,
    current_term: Term,
    current_leader: Option<NodeId>,
    voted_for: Option<NodeId>,
    commit_index: usize,
    votes_responded: FxHashSet::<NodeId>,
    votes_granted: FxHashSet::<NodeId>,
    next_index: FxHashMap::<NodeId, usize>,
    match_index: FxHashMap::<NodeId, usize>,
    // last_log_index: u64,
    // last_log_term: Term,
    // last_applied: u64,
    config: Config,
    // The server has executed the init method
    initialized: bool
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

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum State {
    Follower,
    Candidate,
    Leader
}

#[derive(Hash, PartialEq, Eq)]
pub enum Message {
    RequestVoteRequest(RequestVoteRequestPayload),
    AppendEntriesRequest(AppendEntriesRequestPayload),
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct RequestVoteRequestPayload {
    term: Term, 
    last_log_term: Term, 
    last_log_index: LogIndex, 
    source: NodeId, 
    dest: NodeId
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct AppendEntriesRequestPayload {
    term: Term, 
    prev_log_term: Term, 
    prev_log_index: LogIndex,
    entry: usize, 
    commit_index: LogIndex,
    source: NodeId, 
    dest: NodeId
}

impl RaftNode{
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            log: Log::new(),
            commit_index: 0,
            state: State::Follower,
            current_term: 0,
            current_leader: None,
            voted_for: None,
            votes_responded: FxHashSet::default(),
            votes_granted: FxHashSet::default(),
            next_index: FxHashMap::default(),
            match_index: FxHashMap::default(),
            // last_log_index: 0,
            // last_log_term: 0
            // last_applied: 0,
            config: FxHashSet::default(),
            initialized: false,
        }
    }
    
    pub fn init(&mut self, config: &Config){
        self.config = config.clone();

        for nid in &self.config {
            self.next_index.insert(nid.clone(), 1);
            self.match_index.insert(nid.clone(), 0);
        }
    }

    #[requires(self.is_initialized())]
    #[requires(!self.is_leader())]
    pub fn timeout(&mut self){
        //println!("Node {0} timeout", self.id);
        match self.state {
            State::Follower => self.become_candidate(),
            State::Candidate => self.become_candidate(),
            _ => unreachable!()
        }
    }

    #[requires(self.is_initialized())]
    #[requires(self.is_candidate())]
    pub fn request_vote(&self) -> FxHashSet<Message> {
        let mut messages = FxHashSet::<Message>::default();
        
        for destid in &self.config {
            messages.insert(Message::RequestVoteRequest(RequestVoteRequestPayload{   
                                                            term: self.current_term, 
                                                            last_log_term: last_term(&self.log), 
                                                            last_log_index: 0,
                                                            dest: destid.clone(),
                                                            source: self.id
                                                        }));
        }
        messages
    }

    #[requires(self.is_initialized())]
    #[requires(self.is_leader())]
    pub fn append_entries(&self) -> FxHashSet<Message> {
        let mut messages = FxHashSet::<Message>::default();

        for destid in &self.config {

            let m_prev_log_index = self.next_index.get(destid).unwrap()-1;
            let mut m_prev_log_term = 0;
            if m_prev_log_index > 0 {
                m_prev_log_term = self.log.lookup(m_prev_log_index).term;
            }
            
            let last_entry = cmp::min(self.log.len(), self.next_index.get(destid).unwrap().clone());
            let c_index =  cmp::min(last_entry, self.commit_index);
            
            let m_entry = self.log.lookup(last_entry).entry;

            messages.insert(Message::AppendEntriesRequest(
                                        AppendEntriesRequestPayload{   term: self.current_term, 
                                            prev_log_term: m_prev_log_term, 
                                            prev_log_index: m_prev_log_index,
                                            entry: m_entry.clone(),
                                            commit_index: c_index,
                                            dest: destid.clone(),
                                            source: self.id
                                        }
                                    ));
        }
        messages
    }

    #[requires(self.is_initialized())]
    pub fn receive_message(&mut self, m: Message){
        match m {
            Message::RequestVoteRequest(payload) => {
                self.update_term(payload.term);
                self.handle_requestvoterequest(payload);
            },
            Message::AppendEntriesRequest(payload) => {
                self.update_term(payload.term);
                self.handle_appendentriesrequest(payload);
            }
        }
    }

    #[pure]
    pub fn is_follower(&self) -> bool{
        matches!(self.state,State::Follower)
    }

    #[pure]
    pub fn is_candidate(&self) -> bool{
        matches!(self.state,State::Candidate)
    }

    #[pure]
    pub fn is_leader(&self) -> bool{
        matches!(self.state,State::Leader)
    }

    #[pure]
    pub fn is_initialized(&self) -> bool{
        matches!(self.initialized,true)
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

    fn handle_requestvoterequest(&mut self, m: RequestVoteRequestPayload){
        let log_ok =  m.last_log_term > last_term(&self.log) || 
                      (m.last_log_term == last_term(&self.log) && m.last_log_index >= self.log.len());

        let grant_vote =    m.term == self.current_term 
                            && log_ok 
                            && (matches!(self.voted_for,None) || self.voted_for.unwrap() == m.source);
    }

    fn handle_appendentriesrequest(&self, m: AppendEntriesRequestPayload){
        
    }

    // We jump the term if necessary
    fn update_term(&mut self, newterm: Term){
        if newterm > self.current_term {
            self.current_term = newterm;
            self.state = State::Follower;
            self.voted_for = None;
        }
    }
}

#[pure]
fn last_term(log: &Log) -> Term {
    match log.lookup(log.len()) {
        Empty => 0,
        LogEntry{term, entry} => { 
            term
        }
    }
}