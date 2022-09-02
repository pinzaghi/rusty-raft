use std::option::Option;
use std::cmp;
use std::hash::Hash;

use fxhash::{FxHashSet, FxHashMap};
use parse_display::{Display};

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
    id: NodeId,
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

#[derive(PartialEq, Eq, Display)]
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

    #[requires(is_initialized(&self.match_index, &self.config))]
    pub fn receive_message(&mut self, m: Message) -> Option<Message> {
        assert!(self.initialized);

        let mut result = None;

        match m {
            Message::RequestVoteRequest(payload) => {
                result = self.handle_requestvoterequest(payload);
            },
            Message::RequestVoteResponse(payload) => {
                self.handle_requestvoteresponse(payload);
            },
            Message::AppendEntriesRequest(payload) => {
                result = self.handle_appendentriesrequest(payload);
            },
            Message::AppendEntriesResponse(payload) => {
                self.handle_appendentriesresponse(payload);
            },
        }

        result
    }

    // #############################################
    // Message handlers
    // #############################################
    fn handle_requestvoterequest(&mut self, m: RequestVoteRequestPayload) -> Option<Message> {
        
        self.update_term(m.term);

        let mut result = None;

        if m.term <= self.current_term {
            let log_ok = m.last_log_term > Self::last_term(&self.log) || 
                        (m.last_log_term == Self::last_term(&self.log) && m.last_log_index >= self.log.len());

            let grant_vote = m.term == self.current_term 
                            && log_ok 
                            && (matches!(self.voted_for,None) || self.voted_for.unwrap() == m.source);

            if grant_vote {
                self.voted_for = Some(m.source);
            }

            result = Some(Message::RequestVoteResponse(
                            RequestVoteResponsePayload{
                                        term: self.current_term,
                                        vote_granted: grant_vote,
                                        source: self.id,
                                        dest: Address::Node(m.source.clone())
                                    }
                                )
                    );
        }

        result

    }
    
    fn handle_requestvoteresponse(&mut self, m: RequestVoteResponsePayload){

        self.update_term(m.term);

        if m.term == self.current_term {
            self.votes_responded.insert(m.source);

            if m.vote_granted {
                self.votes_granted.insert(m.source);
            }

            self.become_leader();
        }
    }

    fn handle_appendentriesrequest(&mut self, m: AppendEntriesRequestPayload) -> Option<Message> {

        let mut result = None;

        self.update_term(m.term);

        if m.term <= self.current_term {
        
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
            // reject request
            }else if reject_append {
                result = Some(Message::AppendEntriesResponse(
                                    AppendEntriesResponsePayload{
                                        term: self.current_term,
                                        success: false,
                                        match_index: 0,
                                        source: self.id,
                                        dest: Address::Node(m.source.clone())
                                    }
                        ));
            // accept request
            }else if accept_append{
                //already done with request
                let index = m.prev_log_index+1;
                let done_with_request = matches!(m.entry, None) || 
                                            (matches!(m.entry, Some(..)) &&
                                            self.log.len() >= index && 
                                            self.log.lookup(index).term == m.entry.unwrap().term);

                // conflict: remove 1 entry
                let conflict = matches!(m.entry, Some(..)) && 
                                    self.log.len() >= index && 
                                    self.log.lookup(index).term != m.entry.unwrap().term;

                // no conflict: append entry
                let no_conflict = matches!(m.entry, Some(..)) && 
                                        self.log.len() == m.prev_log_index;

                if done_with_request {
                    self.commit_index = m.commit_index;

                    let mut msg_entries_len = 0;
                    if matches!(m.entry, Some(..)) {
                        msg_entries_len = 1;
                    }

                    result = Some(Message::AppendEntriesResponse(
                                    AppendEntriesResponsePayload{
                                                    term: self.current_term,
                                                    success: true,
                                                    match_index: m.prev_log_index+msg_entries_len,
                                                    source: self.id,
                                                    dest: Address::Node(m.source.clone())
                                                }
                                    ));
                }else if conflict {
                    self.log.pop();
                }else if no_conflict {
                    self.log.push(m.entry.unwrap());
                }
                
            }

            if self.is_leader() {
                self.advance_commit_index();
            }

        }

        result
        
    }

    fn handle_appendentriesresponse(&mut self, m: AppendEntriesResponsePayload){

        self.update_term(m.term);

        if m.term == self.current_term {
            if m.success {
                self.next_index.insert(m.source, m.match_index+1);
                self.match_index.insert(m.source, m.match_index);
            }else{
                let old_next_index = self.next_index[&m.source];

                self.next_index.insert(m.source, cmp::max(old_next_index-1, 1));
            }
        }

        if self.is_leader() {
            self.advance_commit_index();
        }
    }

    // #############################################
    // Actions
    // #############################################

    #[requires(is_initialized(&self.match_index, &self.config))]
    #[requires(!self.is_leader())]
    pub fn timeout(&mut self) -> Message {
        assert!(self.initialized);

        match self.state {
            State::Follower => self.become_candidate(),
            State::Candidate => self.become_candidate(),
            _ => unreachable!()
        }

        self.request_vote()
    }

    pub fn restart(&mut self){
        assert!(self.initialized);
        
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
        assert!(self.initialized);
        assert!(self.is_leader());

        self.log.push(LogEntry{term: self.current_term, value: v});
    }

    #[requires(is_initialized(&self.match_index, &self.config))]
    #[requires(self.is_candidate())]
    pub fn request_vote(&self) -> Message {
        assert!(self.initialized);

        Message::RequestVoteRequest(
                    RequestVoteRequestPayload{   
                        term: self.current_term, 
                        last_log_term: Self::last_term(&self.log), 
                        last_log_index: 0,
                        dest: Address::Broadcast,
                        source: self.id
                    }
                )
    }

    #[requires(is_initialized(&self.next_index, &self.config))]
    #[requires(self.is_leader())]
    #[requires(self.is_valid_id(destid))]
    pub fn append_entries(&mut self, destid: &NodeId) -> Message {
        assert!(self.initialized);
        assert!(self.next_index.contains_key(destid));
        assert!(self.is_leader());

        let m_prev_log_index = get_and_unwrap(&self.next_index, &destid)-1;
        let mut m_prev_log_term = 0;
        if m_prev_log_index > 0 {
            m_prev_log_term = self.log.lookup(m_prev_log_index).term;
        }
        let dest_next_index = *get_and_unwrap(&self.next_index, &destid);
        let last_entry = cmp::min(self.log.len(), dest_next_index);
        let c_index =  cmp::min(last_entry, self.commit_index);

        let mut m_entry = None;

        // We send at most 1 entry
        if dest_next_index == last_entry {
            m_entry = Some(self.log.lookup(last_entry));
        }
        
        Message::AppendEntriesRequest(
                    AppendEntriesRequestPayload{   
                        term: self.current_term, 
                        prev_log_term: m_prev_log_term, 
                        prev_log_index: m_prev_log_index,
                        entry: m_entry,
                        commit_index: c_index,
                        dest: Address::Broadcast,
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
    }

    fn become_leader(&mut self){
        assert!(self.is_candidate() || self.is_leader());

        if self.is_candidate() && self.is_quorum(&self.votes_granted) {
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

        // 1..n+1 iterate n elements
        for idx in 1..self.log.len()+1 {
            let mut agree_set = FxHashSet::<NodeId>::default();

            agree_set.insert(self.id);

            for (nid, node_match_idx) in &self.match_index {
                if node_match_idx >= &idx {
                    agree_set.insert(nid.clone());
                }
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

    fn is_quorum(&self, node_set: &FxHashSet<NodeId>) -> bool {
        assert!(node_set.is_subset(&self.config), "{0} is not subset of {1}", node_set.len(), self.config.len());
        node_set.len() > self.config.len()/2
    }

    #[pure]
    fn last_term(log: &Log) -> Term {
        if log.len() > 0 {
            match log.lookup(log.len()-1) {
                LogEntry{term, value: _} => { 
                    term
                }
            }
        }else{
            0
        }
    }

    // Functions for testing
    pub fn current_term(&self) -> Term {
        self.current_term
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn log(&self) -> Log {
        self.log.clone()
    }

    pub fn commit_index(&self) -> LogIndex {
        self.commit_index
    }
    
}

