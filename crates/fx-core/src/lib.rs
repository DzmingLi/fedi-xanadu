pub mod db;
pub mod error;
pub mod models;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
