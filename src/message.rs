use crate::{raft::{NodeId, Term, LogIndex, Value}, log::LogEntry};

pub enum Message {
    RequestVoteRequest(RequestVoteRequestPayload),
    RequestVoteResponse(RequestVoteResponsePayload),
    AppendEntriesRequest(AppendEntriesRequestPayload),
    AppendEntriesResponse(AppendEntriesResponsePayload),
}

pub struct RequestVoteRequestPayload {
    pub term: Term, 
    pub last_log_term: Term, 
    pub last_log_index: LogIndex, 
    pub source: NodeId, 
    pub dest: NodeId
}

pub struct RequestVoteResponsePayload {
    pub term: Term, 
    pub vote_granted: bool,
    pub source: NodeId, 
    pub dest: NodeId
}

pub struct AppendEntriesRequestPayload {
    pub term: Term, 
    pub prev_log_term: Term, 
    pub prev_log_index: LogIndex,
    pub entry: Option<LogEntry>, 
    pub commit_index: LogIndex,
    pub source: NodeId, 
    pub dest: NodeId
}

pub struct AppendEntriesResponsePayload {
    pub term: Term, 
    pub success: bool, 
    pub match_index: LogIndex,
    pub source: NodeId, 
    pub dest: NodeId
}