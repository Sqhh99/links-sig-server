//! Types module - Request/Response DTOs and error handling
//!
//! This module contains all data transfer objects used in the API:
//! - `requests`: Incoming request body structures
//! - `responses`: Outgoing response body structures  
//! - `error`: Unified error type with HTTP response mapping

pub mod error;
pub mod requests;
pub mod responses;

pub use error::AppError;
pub use requests::*;
pub use responses::*;
