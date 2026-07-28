#[cfg(feature = "message")]
pub mod message;

#[cfg(feature = "resolver")]
pub mod cache;
#[cfg(feature = "resolver")]
pub mod net;
#[cfg(feature = "resolver")]
pub mod resolver;
#[cfg(feature = "resolver")]
pub mod root_hints;
#[cfg(feature = "resolver")]
pub mod singleflight;
