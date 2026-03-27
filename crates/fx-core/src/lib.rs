pub mod db;
pub mod error;
pub mod models;
pub mod region;
pub mod services;
pub mod validation;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
