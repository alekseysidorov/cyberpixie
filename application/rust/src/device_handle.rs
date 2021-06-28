use qmetaobject::*;

#[allow(non_snake_case)]
#[derive(Default, QObject)]
pub struct DeviceHandle {
    base: qt_base_class!(trait QObject),
}
