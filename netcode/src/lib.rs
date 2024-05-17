pub use tarpc;
pub use serde;

use thiserror::Error;
use std::{fmt, net::Ipv4Addr};
use serde::{Deserialize, Serialize};

pub const MAX_FRAME_LENGTH: usize = 8 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XallianceId(pub i64);
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiscordId(pub i64);
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct PrivateUuid(pub String);
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct PrivateUsid(pub String);

impl fmt::Display for XallianceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl fmt::Display for DiscordId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<@{}>", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum NetcodeError {
    #[error("{0}")]
    Internal(String),
    #[error("{0}")]
    Expected(String),
}

pub type NetcodeResult<T> = Result<T, NetcodeError>;

#[tarpc::service]
pub trait MasterApi {
    async fn join(uuid: PrivateUuid, usid: PrivateUsid, name: String, addr: Ipv4Addr) -> NetcodeResult<(XallianceId, Option<String>)>;
    async fn leave(xaid: XallianceId);
    async fn message(xaid: XallianceId, cmd: String);
}

#[tarpc::service]
pub trait GameApi {
    async fn kick(xaid: XallianceId, reason: String) -> NetcodeResult<()>;
    async fn message(xaid: XallianceId, msg: String) -> NetcodeResult<()>;
    async fn broadcast(msg: String) -> NetcodeResult<()>;
}

