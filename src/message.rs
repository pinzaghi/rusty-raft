use parse_display::{Display};

use crate::{raft::{NodeId, Term, LogIndex}, log::LogEntry};

#[derive(Clone, Display)]
pub enum Message {
    #[display("{}{0}")]
    RequestVoteRequest(RequestVoteRequestPayload),
    #[display("{}{0}")]
    RequestVoteResponse(RequestVoteResponsePayload),
    #[display("{}{0}")]
    AppendEntriesRequest(AppendEntriesRequestPayload),
    #[display("{}{0}")]
    AppendEntriesResponse(AppendEntriesResponsePayload),
}

#[derive(Clone, Display)]
pub enum Address {
    Broadcast,
    #[display("Node")]
    Node(NodeId)
}

#[derive(Clone, Display)]
#[display("(term: {term}, last_log_term: {last_log_term}, last_log_index: {last_log_index}, s: {source}, d: {dest})")]
pub struct RequestVoteRequestPayload {
    pub term: Term, 
    pub last_log_term: Term, 
    pub last_log_index: LogIndex, 
    pub source: NodeId, 
    pub dest: Address
}

#[derive(Clone, Display)]
#[display("(term: {term}, vote_granted: {vote_granted}, s: {source}, d: {dest})")]
pub struct RequestVoteResponsePayload {
    pub term: Term, 
    pub vote_granted: bool,
    pub source: NodeId, 
    pub dest: Address
}

#[derive(Clone, Display)]
#[display("(term: {term}, prev_log_term: {prev_log_term}, prev_log_index: {prev_log_index}, commit_index: {commit_index}, s: {source}, d: {dest})")]
pub struct AppendEntriesRequestPayload {
    pub term: Term, 
    pub prev_log_term: Term, 
    pub prev_log_index: LogIndex,
    pub entry: Option<LogEntry>, 
    pub commit_index: LogIndex,
    pub source: NodeId, 
    pub dest: Address
}

#[derive(Clone, Display)]
#[display("(term: {term}, success: {success}, match_index: {match_index}, s: {source}, d: {dest})")]
pub struct AppendEntriesResponsePayload {
    pub term: Term, 
    pub success: bool, 
    pub match_index: LogIndex,
    pub source: NodeId, 
    pub dest: Address
}