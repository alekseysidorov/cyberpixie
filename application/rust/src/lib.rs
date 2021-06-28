use cstr::cstr;
use qmetaobject::prelude::*;

mod device_handle;

#[no_mangle]
extern "C" fn register_cyberpixie_qml_types() {
    println!("Registering Cyberpixie qml types");

    qml_register_type::<device_handle::DeviceHandle>(cstr!("cyberpixie"), 1, 0, cstr!("DeviceHandle"));
}
