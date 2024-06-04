use jni::sys::{jboolean, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use jni::objects::{JClass, JObject, JStaticMethodID, JString, JValue, JValueGen};
use xalliance_netcode::{PlayerName, PlayerUuid};

use crate::{core, AppResult};

fn get_string<'local>(
    env: &mut JNIEnv<'local>,
    object: &JObject<'local>,
    name: &str) -> String {
    let JValueGen::Object(value) = env.get_field(object, name, "Ljava/lang/String;")
        .expect(&*format!("get_field {name} failed")) else { unreachable!() };
    env .get_string(&value.into())
        .expect("get_string failed")
        .into()
}

#[no_mangle]
pub extern "system" fn Java_mindustry_XAlliance_init<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>) {
    core();
}

#[no_mangle]
pub extern "system" fn Java_mindustry_XAlliance_handleServerReceived<'local>(
    mut env: JNIEnv<'local>,
    xalliance_class: JClass<'local>,
    connection: JObject<'local>,
    packet: JObject<'local>) -> jboolean {
    let packet_class = env
        .get_object_class(&packet)
        .expect("packet class not found");
    let JValueGen::Object(packet_name) = env
        .call_method(&packet_class, "getName", "()Ljava/lang/String;", &[])
        .expect("getName call failed") else { unreachable!() };
    let packet_name: String = env
        .get_string(&packet_name.into())
        .expect("get string failed")
        .into();

    let ret = match &*packet_name {
        "mindustry.gen.ClientSnapshotCallPacket" |
        "mindustry.gen.PingCallPacket" =>
            return JNI_FALSE,
        "mindustry.net.Packets$ConnectPacket" => {
            let uuid = get_string(&mut env, &packet, "uuid");
            let name = get_string(&mut env, &packet, "name");
            let addr = get_string(&mut env, &connection, "address");
            let jvm = env.get_java_vm().unwrap();
            let xalliance_class = env.new_global_ref(xalliance_class).unwrap();
            let connection = env.new_global_ref(connection).unwrap();
            let packet = env.new_global_ref(packet).unwrap();
            core().spawn(async move {
                // TODO: prevent recursion
                let res: AppResult<_> = async move {
                    let action = core().master().await?.join(
                        tarpc::context::current(),
                        PlayerUuid(uuid),
                        PlayerName(name),
                        addr.parse().expect("ip parse failed")
                    ).await??;
                    Ok(action)
                }.await;

                let mut env = jvm.attach_current_thread_as_daemon().expect("jvm thread attach failed");
                match res {
                    Ok(name) => {
                        println!("new player name received: {name}");
                        let name = env
                            .new_string(name.to_string())
                            .expect("create string failed");
                        env.set_field(&packet, "name", "Ljava/lang/String;", JValue::from(&name))
                            .expect("set name failed");
                    },
                    Err(e) => eprintln!("err {}", e),
                }
                env.call_static_method(
                    &xalliance_class,
                    "handleServerReceivedInjected",
                    "(Lmindustry/net/NetConnection;Lmindustry/net/Packet;)V",
                    &[JValue::from(&connection),JValue::from(&packet)]
                ).expect("injectPacket call failed");
            });
            JNI_TRUE
        },
        _ => JNI_FALSE
    };
    return ret;
}
