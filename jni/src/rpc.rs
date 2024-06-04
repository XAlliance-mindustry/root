use std::{io, net::SocketAddr, time::Duration};
use futures::{future::{join, JoinAll}, prelude::*};
use tarpc::{self, client::RpcError, server::Channel, transport::channel::UnboundedChannel, ClientMessage, Response};
use thiserror::Error;
use tokio::{sync::{RwLock, RwLockReadGuard}, task::JoinHandle};
use xalliance_netcode::{spawn_twoway, GameApi, GameApiRequest, GameApiResponse, MasterApiClient, MasterApiRequest, MasterApiResponse, NetcodeError, NetcodeResult, XallianceId, MAX_FRAME_LENGTH};

#[derive(Clone)]
pub struct Session {
}

impl GameApi for Session {
    async fn kick(self,context: ::tarpc::context::Context,xaid:XallianceId,reason:String) -> NetcodeResult<()>  {
        todo!()
    }

    async fn message(self,context: ::tarpc::context::Context,xaid:XallianceId,msg:String) -> NetcodeResult<()>  {
        todo!()
    }

    async fn broadcast(self,context: ::tarpc::context::Context,msg:String) -> NetcodeResult<()>  {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("remote: {0}")]
    Remote(#[from] NetcodeError),
    #[error("connection failed: {0}")]
    ConnectionFailed(#[source] io::Error),
    #[error("disconnected")]
    Disconnected,
    #[error("rpc failed: {0}")]
    Rpc(#[from] RpcError),
}

pub type AppResult<T> = std::result::Result<T, AppError>;

pub struct NetHandler {
    master_api: RwLock<Option<MasterApiClient>>,
}

impl NetHandler {
    pub fn new() -> Self {
        Self { master_api: RwLock::new(None) }
    }

    pub async fn connect(&self, master_endpoint: SocketAddr, server_name: String) -> AppResult<impl Future<Output = ()>> {
        let mut connect = tarpc::serde_transport::tcp::connect(master_endpoint, tarpc::tokio_serde::formats::Json::default);
        connect.config_mut().max_frame_length(MAX_FRAME_LENGTH);
        let transport = connect.await.map_err(|e| AppError::ConnectionFailed(e))?;
        let (server_transport, client_transport) = spawn_twoway(transport);
        let channel = tarpc::server::BaseChannel::new(tarpc::server::Config::default(), server_transport);
        let session = Session {};
        let server_handle = tokio::spawn(channel
            .execute(session.serve())
            .for_each(|response| async {
                tokio::spawn(response);
            })
        );

        let client_handle = tokio::spawn({
            let mut master_api = self.master_api.write().await;
            let client = MasterApiClient::new(tarpc::client::Config::default(), client_transport);
            *master_api = Some(client.client);
            client.dispatch.unwrap_or_else(move |e| {
                println!("connection broken: {e}");
            })
        });
        println!("connected to master");

        self.master()
            .await?
            .register(tarpc::context::current(), server_name)
            .await??;
        println!("registred on master");
        
        let handles = future::join(server_handle, client_handle);
        Ok(handles.map(|_|{}))
    }

    pub async fn master(&self) -> AppResult<RwLockReadGuard<MasterApiClient>> {
        let lock = self.master_api.read().await;
        RwLockReadGuard::try_map(lock, |x| x.as_ref())
            .map_err(|_| AppError::Disconnected)
    }
}
