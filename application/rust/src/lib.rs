use cstr::cstr;
use log::LevelFilter;
use qmetaobject::prelude::*;

mod device_handle;

#[no_mangle]
extern "C" fn register_cyberpixie_qml_types() {
    #[cfg(not(target_os = "android"))]
    env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .init();
    #[cfg(target_os = "android")]
    android_logger::init_once(android_logger::Config::default().with_max_level(LevelFilter::Trace));

    qmetaobject::log::init_qt_to_rust();

    log::info!("Registering Cyberpixie qml types");

    qml_register_type::<device_handle::DeviceHandle>(
        cstr!("cyberpixie"),
        1,
        0,
        cstr!("DeviceHandle"),
    );
}
