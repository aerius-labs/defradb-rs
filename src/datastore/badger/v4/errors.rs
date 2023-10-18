use thiserror::Error;

#[derive(Error, Debug)]
pub enum BadgerError {

    #[error("invalid order type: {0}")]
    InvalidOrderType(String)
}