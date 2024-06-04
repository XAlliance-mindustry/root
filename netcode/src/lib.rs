use futures::{stream::{AbortHandle, Abortable}, Sink, SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use thiserror::Error;
use std::{fmt, io, net::Ipv4Addr, result};
use serde::{Deserialize, Serialize};
use tarpc::{self, client::RpcError, transport::channel::{ChannelError, UnboundedChannel}};

pub const MAX_FRAME_LENGTH: usize = 8 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XallianceId(pub i64);
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiscordId(pub i64);
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct PlayerUuid(pub String);
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct PlayerName(pub String);
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct DiscordLogin(pub String);

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

impl fmt::Display for DiscordLogin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

impl fmt::Display for PlayerName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum NetcodeError {
    #[error("{0}")]
    Internal(String),
    #[error("{0}")]
    Expected(String),
}

pub type NetcodeResult<T> = std::result::Result<T, NetcodeError>;

#[tarpc::service]
pub trait MasterApi {
    async fn register(server: String) -> NetcodeResult<()>;
    async fn join(uuid: PlayerUuid, name: PlayerName, addr: Ipv4Addr) -> NetcodeResult<String>;
    async fn leave(xaid: XallianceId);
    async fn message(xaid: XallianceId, cmd: String);
}

#[tarpc::service]
pub trait GameApi {
    async fn kick(xaid: XallianceId, reason: String) -> NetcodeResult<()>;
    async fn message(xaid: XallianceId, msg: String) -> NetcodeResult<()>;
    async fn broadcast(msg: String) -> NetcodeResult<()>;
}

/// A tarpc message that can be either a request or a response.
#[derive(Serialize, Deserialize)]
pub enum TwoWayMessage<Req, Resp> {
    ClientMessage(tarpc::ClientMessage<Req>),
    Response(tarpc::Response<Resp>),
}

#[derive(Debug, Error)]
enum ChannelOrIoError {
    #[error("{0}")]
    ChannelError(#[from] ChannelError),
    #[error("{0}")]
    IoError(#[from] io::Error),
}

pub fn spawn_twoway<Req1, Resp1, Req2, Resp2, T>(
    transport: T,
) -> (
    UnboundedChannel<tarpc::ClientMessage<Req1>, tarpc::Response<Resp1>>,
    UnboundedChannel<tarpc::Response<Resp2>, tarpc::ClientMessage<Req2>>,
)
where
    T: Stream<Item = result::Result<TwoWayMessage<Req1, Resp2>, io::Error>>,
    T: Sink<TwoWayMessage<Req2, Resp1>, Error = io::Error>,
    T: Unpin + Send + 'static,
    Req1: Send + 'static,
    Resp1: Send + 'static,
    Req2: Send + 'static,
    Resp2: Send + 'static,
{
    let (server, server_ret) = tarpc::transport::channel::unbounded();
    let (client, client_ret) = tarpc::transport::channel::unbounded();
    let (mut server_sink, server_stream) = server.split();
    let (mut client_sink, client_stream) = client.split();
    let (transport_sink, mut transport_stream) = transport.split();

    let (abort_handle, abort_registration) = AbortHandle::new_pair();

    // Task for inbound message handling
    tokio::spawn(async move {
        let e: result::Result<(), ChannelOrIoError> = async move {
            while let Some(msg) = transport_stream.next().await {
                match msg? {
                    TwoWayMessage::ClientMessage(req) => server_sink.send(req).await?,
                    TwoWayMessage::Response(resp) => client_sink.send(resp).await?,
                }
            }
            Ok(())
        }
        .await;

        match e {
            Ok(()) => println!("transport_stream done"),
            Err(e) => eprintln!("Error in inbound multiplexing: {}", e),
        }

        abort_handle.abort();
    });

    let abortable_sink_channel = Abortable::new(
        futures::stream::select(
            server_stream.map_ok(TwoWayMessage::Response),
            client_stream.map_ok(TwoWayMessage::ClientMessage),
        )
        .map_err(ChannelOrIoError::ChannelError),
        abort_registration,
    );

    // Task for outbound message handling
    tokio::spawn(
        abortable_sink_channel
            .forward(transport_sink.sink_map_err(ChannelOrIoError::IoError))
            .inspect_ok(|_| println!("transport_sink done"))
            .inspect_err(|e| eprintln!("Error in outbound multiplexing: {}", e))
    );

    (server_ret, client_ret)
}
