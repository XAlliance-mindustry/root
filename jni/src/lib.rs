use jni::JNIEnv;
use jni::objects::{JClass, JObject, JStaticMethodID, JString, JValue, JValueGen};

#[no_mangle]
pub extern "system" fn Java_mindustry_XAlliance_handleServerReceived<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    _connection: JObject<'local>,
    packet: JObject<'local>) {
    let xalliance_class = env
        .find_class("mindustry/XAlliance")
        .expect("XAlliance class not found");
    let packet_class = env
        .get_object_class(&packet)
        .expect("packet class not found");
    let JValueGen::Object(name) = env
        .call_method(&packet_class, "getName", "()Ljava/lang/String;", &[])
        .expect("getName call failed")
    else {
        unreachable!()
    };
    let name: String = env
        .get_string(&name.into())
        .expect("get string failed")
        .into();
    match &*name {
        "mindustry.gen.ClientSnapshotCallPacket"
        | "mindustry.gen.PingCallPacket"
            => return,
        "mindustry.net.Packets$ConnectPacket"
            => {
            let JValueGen::Object(uuid) = env.get_field(&packet, "uuid", "Ljava/lang/String;")
                .expect("get uuid failed")
            else {
                unreachable!()
            };
            let uuid: String = env
                .get_string(&uuid.into())
                .expect("create string failed")
                .into();
            let uuid = env
                .new_string(format!("__{uuid}"))
                .expect("create string failed");
            env.set_field(&packet, "uuid", "Ljava/lang/String;", JValue::from(&uuid))
                .expect("set uuid failed");
        },
        _ => {}
    }
    let output = env
        .new_string(format!("Packet hooked: {name}"))
        .expect("create string failed");
    env.call_static_method(&xalliance_class, "info", "(Ljava/lang/String;)V", &[JValue::from(&output)])
        .expect("info call failed");
}
