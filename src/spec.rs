use std::collections::{HashSet, HashMap};
use std::borrow::Borrow;
use std::hash::Hash;
use std::hash::BuildHasher;

use prusti_contracts::*;

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

    #[trusted]
    #[pure]
    #[ensures(result ==> matches!(self.get(value), Some(value)))]
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq;

    #[trusted]
    #[pure]
    pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Hash + Eq;
}

#[extern_spec]
impl<K, V, S> HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    #[trusted]
    #[pure]
    #[ensures(result ==> matches!(self.get(k), Some(_)))]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq;

    #[trusted]
    #[pure]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq;
}

#[extern_spec]
impl<K, V, S> HashMap<K, V, S> {
    #[pure]
    pub fn len(&self) -> usize;
}

#[extern_spec]
mod std {
    mod cmp {
        use prusti_contracts::*;
        
        //#[ensures(result >= v1 && result >= v2)]
        pub fn min<T: Ord>(v1: T, v2: T) -> T;
    }
}