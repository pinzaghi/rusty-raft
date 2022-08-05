use crate::raft::NodeId;

pub type Address = String;
pub type ConfigEntry = (NodeId, Address);

#[derive(Clone)]
pub struct Config {
    members: Vec<ConfigEntry>
}

impl Config {
    pub fn new(members: Vec<ConfigEntry>) -> Config {
        Config {
            members
        }
    }
}