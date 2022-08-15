/*
* ./prusti-rustc -Pcheck_overflows=false --edition=2021 src/utils/list.rs
*/
use std::mem;

use prusti_contracts::*;

use crate::raft::Term;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct LogEntry {
    pub term: Term, 
    pub entry: usize
}

pub enum TrustedOption {
    Some(LogEntry),
    None,
}

impl TrustedOption {

    #[pure]
    pub fn is_none(&self) -> bool {
        match self {
            TrustedOption::Some(_) => false,
            TrustedOption::None => true,
        }
    }

    #[pure]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    #[pure]
    #[requires(self.is_some())]
    pub fn peek(&self) -> LogEntry {
        match self {
            TrustedOption::Some(val) => *val,
            TrustedOption::None => unreachable!(),
        }
    }

}

pub struct Log {
    head: Link,
}

enum Link {
    Empty,
    More(Box<Node>),
}

struct Node {
    elem: LogEntry,
    next: Link,
}

#[trusted]
#[requires(src.is_empty())]
#[ensures(dest.is_empty())]
#[ensures(old(dest.len()) == result.len())]
#[ensures(forall(|i: usize| (0 <= i && i < result.len()) ==> 
                old(dest.lookup(i)) == result.lookup(i)))] 
fn replace(dest: &mut Link, src: Link) -> Link {
    mem::replace(dest, src)
}


impl Log {

    #[pure]
    pub fn len(&self) -> usize {
        self.head.len()
    }

    #[pure]
    #[requires(0 <= index && index < self.len())]
    pub fn lookup(&self, index: usize) -> LogEntry {
        self.head.lookup(index)
    }

    #[ensures(result.len() == 0)]
    pub fn new() -> Self {
        Log {
            head: Link::Empty
        }
    }

    #[ensures(self.len() == old(self.len()) + 1)] 
    #[ensures(self.lookup(0) == old(elem))]
    #[ensures(forall(|i: usize| (1 <= i && i < self.len()) ==>
        old(self.lookup(i-1)) == self.lookup(i)))]
    pub fn push(&mut self, elem: LogEntry) {
        let new_node = Box::new(Node {
            elem: elem,
            next: replace(&mut self.head, Link::Empty),
        });

        self.head = Link::More(new_node);
    }

    #[ensures(old(self.len()) == 0 ==> result.is_none())]
    #[ensures(old(self.len()) > 0 ==> result.is_some())]
    #[ensures(old(self.len()) == 0 ==> self.len() == 0)]
    #[ensures(old(self.len()) > 0 ==> self.len() == old(self.len()-1))]
    #[ensures(old(self.len()) > 0 ==> result.peek() == old(self.lookup(0)))]
    #[ensures(old(self.len()) > 0 ==>
    forall(|i: usize| (0 <= i && i < self.len()) ==>
        old(self.lookup(i+1)) == self.lookup(i)))]
    pub fn pop(&mut self) -> TrustedOption {
        match replace(&mut self.head, Link::Empty) {
            Link::Empty => {
                TrustedOption::None
            }
            Link::More(node) => {
                self.head = node.next;
                TrustedOption::Some(node.elem)
            }
        }
    }
}

impl Link {

    #[pure]
    #[ensures(!self.is_empty() ==> result > 0)]
    #[ensures(result >= 0)]
    fn len(&self) -> usize {
        match self {
            Link::Empty => 0,
            Link::More(box node) => 1 + node.next.len(),
        }
    }

    #[pure]
    fn is_empty(&self) -> bool {
        match self {
            Link::Empty => true,
            Link::More(box node) => false,
        }
    }

    #[pure]
    #[requires(0 <= index && index < self.len())]
    pub fn lookup(&self, index: usize) -> LogEntry {
        match self {
            Link::Empty => unreachable!(),
            Link::More(box node) => {
                if index == 0 {
                    node.elem
                } else {
                    node.next.lookup(index - 1)
                }
            }
        }
    }

}