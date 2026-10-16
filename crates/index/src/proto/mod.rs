//! The wire contract between the Neoworks daemon and the applications that
//! embed it. Everything in here is shared by both sides, so a change is only
//! safe when it is additive.

mod framing;
mod methods;
mod paths;
mod protocol;

pub use framing::{MAXIMUM_MESSAGE_BYTES, MessageTooLarge, is_message_too_large, read_message, write_message};
pub use methods::*;
pub use paths::*;
pub use protocol::*;
