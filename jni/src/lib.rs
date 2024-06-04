pub mod jni;
pub mod rpc;

use std::{net::{Ipv4Addr, SocketAddr, SocketAddrV4}, sync::{Arc, OnceLock}, thread, time::Duration};
use futures::prelude::*;
use rpc::*;
use tokio::{sync::RwLockReadGuard, task::JoinHandle};
use xalliance_netcode::MasterApiClient;

pub fn core() -> &'static Core {
    static CORE: OnceLock<Core> = OnceLock::new();
    CORE.get_or_init(Core::new)
}

pub struct Core {
    runtime: Arc<tokio::runtime::Runtime>,
    net: Arc<NetHandler>,
}

impl Core {
    fn new() -> Self {
        let master_endpoint = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 50051));
        // TODO: take directory name
        let server_name = "test".to_owned();
        let runtime = Arc::new(tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed creating tokio runtime"));
        let net = Arc::new(NetHandler::new());
        {
            let runtime = runtime.clone();
            let net = net.clone();
            thread::spawn(move ||
                runtime.block_on(async {
                    loop {
                        // TODO: handle errors
                        net.connect(master_endpoint, server_name.clone())
                            .await
                            .unwrap()
                            .await;
                        tokio::time::sleep(Duration::from_secs(15)).await;
                    }
                })
            );
        }
        Self { runtime, net }
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    pub async fn master(&self) -> AppResult<RwLockReadGuard<MasterApiClient>> {
        self.net.master().await
    }
}
