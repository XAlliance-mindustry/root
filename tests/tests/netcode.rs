use std::{net::{Ipv4Addr, SocketAddr, SocketAddrV4}, sync::Arc, time::Duration};

use xalliance_jni::rpc::AppError;
use xalliance_netcode::{PlayerName, PlayerUuid, XallianceId};


#[tokio::main]
#[test]
async fn join() {
    let core = xalliance_master_server::core::Core::new("test_rpc").await;
    core.drop_database().await.unwrap();
    let server = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 50051));

    let uuid = PlayerUuid("uuid".to_owned());
    let name = PlayerName("name".to_owned());
    let addr = Ipv4Addr::new(1, 1, 1, 1);
    let xaid = XallianceId(1024);

    core.on_player_join(server, uuid.clone(), name.clone(), addr)
        .await.unwrap();
    tokio::spawn(xalliance_master_server::rpc::serve(Arc::new(core), server));

    let await_connection = async {
        while let Err(AppError::Disconnected) = xalliance_jni::core().master().await {}
    };
    if let Err(_) = tokio::time::timeout(Duration::from_secs(1), await_connection).await {
        panic!("connection timeout");
    }

    let new_name = xalliance_jni::core().master()
        .await.unwrap()
        .join(tarpc::context::current(), uuid.clone(), name.clone(), addr)
        .await.unwrap().unwrap();

    assert_eq!(new_name, name.0 + " " + &xaid.to_string());
}
