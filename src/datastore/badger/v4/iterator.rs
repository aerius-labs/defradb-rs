use datastore::{Store, Read, Write, StoreData, DataQuery, Error};
use std::result::Result;

pub trait Iterable {
    fn get_iterator(&self, query: DataQuery) -> Result<Box<dyn Iterator>, Box<dyn Error>>;
}

pub trait Iterator {
    fn iterate_prefix(&self, start_prefix: String, end_prefix: String) -> Result<Vec<Box<dyn StoreData>>, Box<dyn Error>>;
    fn close(&mut self) -> Result<(), Box<dyn Error>>;
}

pub trait IterableTxn: Read + Write + Iterable {}

pub trait IterableDatastore: Store + Iterable {}

pub trait IterableTxnDatastore: Store {
    fn new_iterable_transaction(&self, read_only: bool) -> Result<Box<dyn IterableTxn>, Box<dyn Error>>;
}