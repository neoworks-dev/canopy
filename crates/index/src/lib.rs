//! The file-search side of Canopy: the shared Neoworks index daemon (`canopyd`),
//! the protocol and client applications use to reach it, and the explorer's
//! glue over that client.

pub mod client;
pub mod daemon;
pub mod index;
pub mod lockfile;
pub mod proto;
pub mod searchindex;
pub mod server;
pub mod stop;

#[cfg(test)]
mod test_support;
