#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod avl;
pub mod concurrent_map;
pub mod heap;
pub mod protocol;
pub mod server;
pub mod storage;
pub mod threadpool;
pub mod zset;