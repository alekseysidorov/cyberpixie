use qmetaobject::*;

#[allow(non_snake_case)]
#[derive(Default, QObject)]
pub struct DeviceHandle {
    base: qt_base_class!(trait QObject),
    // And even a slot
    compute_greetings: qt_method!(fn compute_greetings(&self, verb: String) -> QString {
        format!("{} {}", verb, "Cyberpixie").into()
    })
}
