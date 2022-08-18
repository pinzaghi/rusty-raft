use std::option::Option;
use std::cmp;
use std::hash::Hash;

use fxhash::{FxHashSet, FxHashMap};

use crate::log::{Log, LogEntry};
use crate::message::*;

use prusti_contracts::*;

pub type NodeId = usize;
pub type Term = usize;
pub type LogIndex = usize;
pub type Config = FxHashSet<NodeId>;
pub type Value = usize;

// A Raft node
pub struct RaftNode {
    pub id: NodeId,
    log: Log,
    state: State,
    current_term: Term,
    voted_for: Option<NodeId>,
    commit_index: LogIndex,
    votes_responded: FxHashSet::<NodeId>,
    votes_granted: FxHashSet::<NodeId>,
    next_index: FxHashMap::<NodeId, LogIndex>,
    match_index: FxHashMap::<NodeId, LogIndex>,
    config: Config,
    // The server has executed the init method
    initialized: bool
}

#[derive(PartialEq, Eq)]
pub enum State {
    Follower,
    Candidate,
    Leader
}

predicate! { 
    pub fn is_initialized(map: &FxHashMap::<NodeId, LogIndex>, config: &FxHashSet<NodeId>) -> bool { 
        //map.len() == config.len() && forall(|i: usize| (0 <= i && i < config.len()) ==> map.contains_key(&config.get(i))) 
        map.len() == config.len()
    } 
}

#[trusted]
#[requires(map.contains_key(key))]
fn get_and_unwrap<'a, K: Eq + Hash, V>(map: &'a FxHashMap<K, V>, key: &'a K) -> &'a V {
    map.get(key).unwrap()
}

impl RaftNode{
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            log: Log::new(),
            commit_index: 0,
            state: State::Follower,
            current_term: 0,
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
    
    #[trusted]
    #[ensures(is_initialized(&self.next_index, &self.config))]
    #[ensures(is_initialized(&self.match_index, &self.config))]
    pub fn init(&mut self, config: &Config){
        self.config = config.clone();

        for nid in &self.config {
            self.next_index.insert(nid.clone(), 1);
            self.match_index.insert(nid.clone(), 0);
        }
        self.initialized = true;
    }

    // #############################################
    // Message handlers
    // #############################################
    #[requires(is_initialized(&self.match_index, &self.config))]
    pub fn receive_message(&mut self, m: Message){
        match m {
            Message::RequestVoteRequest(payload) => {
                self.update_term(payload.term);

                if payload.term <= self.current_term {
                    self.handle_requestvoterequest(payload);
                }
            },
            Message::RequestVoteResponse(payload) => {
                self.update_term(payload.term);

                if payload.term == self.current_term {
                    self.handle_requestvoteresponse(payload);
                    self.become_leader();
                }
            },
            Message::AppendEntriesRequest(payload) => {
                self.update_term(payload.term);

                if payload.term <= self.current_term {
                    self.handle_appendentriesrequest(payload);
                }
            },
            Message::AppendEntriesResponse(payload) => {
                self.update_term(payload.term);

                if payload.term == self.current_term {
                    self.handle_appendentriesresponse(payload);
                }
            },
        }

        if self.is_leader() {
            self.advance_commit_index();
        }
        
    }

    fn handle_requestvoterequest(&mut self, m: RequestVoteRequestPayload) -> Message {
        let log_ok = m.last_log_term > Self::last_term(&self.log) || 
                     (m.last_log_term == Self::last_term(&self.log) && m.last_log_index >= self.log.len());

        let grant_vote = m.term == self.current_term 
                         && log_ok 
                         && (matches!(self.voted_for,None) || self.voted_for.unwrap() == m.source);

        if grant_vote {
            self.voted_for = Some(m.source);
        }     

        Message::RequestVoteResponse(
                    RequestVoteResponsePayload{
                        term: self.current_term,
                        vote_granted: grant_vote,
                        source: self.id,
                        dest: m.source.clone()
                    }
        )   
    }
    
    fn handle_requestvoteresponse(&mut self, m: RequestVoteResponsePayload){
        if m.term == self.current_term {
            self.votes_responded.insert(m.source);

            if m.vote_granted {
                self.votes_granted.insert(m.source);
            }
        }
    }

    fn handle_appendentriesrequest(&mut self, m: AppendEntriesRequestPayload) -> Option<Message> {

        let log_ok = m.prev_log_index == 0 ||
                           (m.prev_log_index > 0 && 
                            m.prev_log_index <= self.log.len() && 
                            m.prev_log_term == self.log.lookup(m.prev_log_index).term);
        
        let reject_append = m.term < self.current_term || 
                                  (m.term == self.current_term && 
                                   self.is_follower() && 
                                   !log_ok);

        let accept_append = m.term == self.current_term && 
                                  self.is_follower() &&
                                  log_ok;
        
        // return to follower state
        if m.term == self.current_term && self.is_candidate() {
            self.state = State::Follower;
            None
        // reject request
        }else if reject_append {
            Some(Message::AppendEntriesResponse(
                        AppendEntriesResponsePayload{
                            term: self.current_term,
                            success: false,
                            match_index: 0,
                            source: self.id,
                            dest: m.source.clone()
                        }
            ))
        // accept request
        }else if accept_append{
            //already done with request
            let index = m.prev_log_index+1;
            let done_with_request = matches!(m.entry, None) || 
                                          (self.log.len() >= index && 
                                           self.log.lookup(index).term == m.entry.unwrap().term);

            // conflict: remove 1 entry
            let conflict = matches!(m.entry, Some(..)) && 
                                 self.log.len() >= index && 
                                 self.log.lookup(index).term != m.entry.unwrap().term;

            // no conflict: append entry
            let no_conflict = matches!(m.entry, Some(..)) && 
                                    self.log.len() == index;

            if done_with_request {
                self.commit_index = m.commit_index;

                let mut msg_entries_len = 0;
                if matches!(m.entry, Some(..)) {
                    msg_entries_len = 1;
                }

                Some(Message::AppendEntriesResponse(
                    AppendEntriesResponsePayload{
                                    term: self.current_term,
                                    success: true,
                                    match_index: m.prev_log_index+msg_entries_len,
                                    source: self.id,
                                    dest: m.source.clone()
                                }
                    ))
            }else if conflict {
                self.log.pop();
                None
            }else if no_conflict {
                self.log.push(m.entry.unwrap());
                None
            }else{
                None
            }
            
        }else{
            None
        }
        
    }

    fn handle_appendentriesresponse(&mut self, m: AppendEntriesResponsePayload){
        if m.term == self.current_term {
            if m.success {
                self.next_index.insert(m.source, m.match_index+1);
                self.match_index.insert(m.source, m.match_index);
            }else{
                let old_next_index = self.next_index[&m.source];

                self.next_index.insert(m.source, cmp::max(old_next_index-1, 1));
            }
        }
    }

    // #############################################
    // Actions
    // #############################################

    #[requires(is_initialized(&self.match_index, &self.config))]
    #[requires(!self.is_leader())]
    pub fn timeout(&mut self){
        match self.state {
            State::Follower => self.become_candidate(),
            State::Candidate => self.become_candidate(),
            _ => unreachable!()
        }
    }

    pub fn restart(&mut self){
        self.state = State::Follower;
        self.votes_responded.clear();
        self.votes_granted.clear();
        self.commit_index = 0;

        for (_, nidx) in self.next_index.iter_mut() {
            *nidx = 1;
        }

        for (_, nidx) in self.match_index.iter_mut() {
            *nidx = 0;
        }
    }

    #[requires(self.is_leader())]
    pub fn client_request(&mut self, v: Value){
        assert!(self.is_leader());

        self.log.push(LogEntry{term: self.current_term, value: v});
    }

    // This function returns the RequestVote message to be send
    #[requires(is_initialized(&self.match_index, &self.config))]
    #[requires(self.is_candidate())]
    pub fn request_vote(&self, destid: &NodeId) -> Message {
        Message::RequestVoteRequest(
                    RequestVoteRequestPayload{   
                        term: self.current_term, 
                        last_log_term: Self::last_term(&self.log), 
                        last_log_index: 0,
                        dest: destid.clone(),
                        source: self.id
                    }
                )
    }

    #[requires(is_initialized(&self.next_index, &self.config))]
    #[requires(self.is_leader())]
    #[requires(self.is_valid_id(destid))]
    pub fn append_entries(&self, destid: &NodeId) -> Message {
        assert!(self.next_index.contains_key(destid));

        let m_prev_log_index = get_and_unwrap(&self.next_index, &destid)-1;
        let mut m_prev_log_term = 0;
        if m_prev_log_index > 0 {
            m_prev_log_term = self.log.lookup(m_prev_log_index).term;
        }
        
        let last_entry = self.last_entry(destid);
        let c_index =  cmp::min(last_entry, self.commit_index);
        
        let m_entry = self.log.lookup(last_entry);

        Message::AppendEntriesRequest(
                    AppendEntriesRequestPayload{   
                        term: self.current_term, 
                        prev_log_term: m_prev_log_term, 
                        prev_log_index: m_prev_log_index,
                        entry: Some(m_entry.clone()),
                        commit_index: c_index,
                        dest: destid.clone(),
                        source: self.id
                    }
                )
    }

    // #############################################
    // Auxiliary functions
    // #############################################

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

    fn become_leader(&mut self){
        assert!(self.is_candidate());

        if self.is_quorum(&self.votes_granted) {
            self.state = State::Leader;

            // Move next index for each node
            for (_, nidx) in self.next_index.iter_mut() {
                *nidx = self.log.len()+1;
            }

            // Reset match index for each node
            for (_, nidx) in self.match_index.iter_mut() {
                *nidx = 0;
            }
        }
    }

    #[requires(self.is_leader())]
    fn advance_commit_index(&mut self) {

        let mut maybe_max_index : Option<LogIndex> = None;

        for idx in 0..self.log.len() {
            let mut agree_set = FxHashSet::<NodeId>::default();

            for (nid, nidx) in &self.match_index {
                agree_set.insert(nid.clone());
            }

            if self.is_quorum(&agree_set) {
                maybe_max_index = Some(idx);
            }
        }

        match maybe_max_index {
            None => {},
            Some(max_index) => {
                if self.log.lookup(max_index).term == self.current_term{
                    self.commit_index = max_index;
                }
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
    pub fn is_valid_id(&self, id: &NodeId) -> bool{
        self.config.contains(id)
    }

    // We jump the term if necessary
    fn update_term(&mut self, newterm: Term){
        if newterm > self.current_term {
            self.current_term = newterm;
            self.state = State::Follower;
            self.voted_for = None;
        }
    }

    fn last_entry(&self, destid: &NodeId) -> LogIndex {
        assert!(self.next_index.contains_key(destid));

        cmp::min(self.log.len(), *get_and_unwrap(&self.next_index, &destid))
    }

    fn is_quorum(&self, node_set: &FxHashSet<NodeId>) -> bool {
        assert!(node_set.is_subset(&self.config));

        node_set.len() > self.config.len()/2
    }

    #[pure]
    fn last_term(log: &Log) -> Term {
        if log.len() > 0 {
            match log.lookup(log.len()-1) {
                LogEntry{term, value} => { 
                    term
                },
                _ => unreachable!(),
            }
        }else{
            0
        }
    }
}

