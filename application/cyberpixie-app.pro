QT += quick widgets

CONFIG += c++11

# You can make your code fail to compile if it uses deprecated APIs.
# In order to do so, uncomment the following line.
#DEFINES += QT_DISABLE_DEPRECATED_BEFORE=0x060000    # disables all the APIs deprecated before Qt 6.0.0

SOURCES += \
        filesreader.cpp \
        main.cpp

RESOURCES += qml.qrc

# Additional import path used to resolve QML modules in Qt Creator's code model
QML_IMPORT_PATH =

# Additional import path used to resolve QML modules just for Qt Quick Designer
QML_DESIGNER_IMPORT_PATH =

# Default rules for deployment.
qnx: target.path = /tmp/$${TARGET}/bin
else: unix:!android: target.path = /opt/$${TARGET}/bin
!isEmpty(target.path): INSTALLS += target

# Additional target to build binding code with the Cyberpixie Rust part.
RUST_BINDING_FILES += \
    rust/Cargo.toml \
    rust/build.rs \
    rust/src/device_handle.rs \
    rust/src/lib.rs

ios {
#    debug {
#        CARGO_BUILD_PATH = aarch64-apple-ios/debug
#    } else {
#        CARGO_BUILD_PATH = aarch64-apple-ios/release
#        CARGO_EXTRA_ARGS += --release
#    }
    CARGO_BUILD_PATH = aarch64-apple-ios/release
    CARGO_EXTRA_ARGS +=--target aarch64-apple-ios --release

    RUST_BINDING_LIB = $$PWD/../target/$$CARGO_BUILD_PATH/libcyberpixie_qml.a
    message("-----")
    message("build_path: " $$CARGO_BUILD_PATH)
    message("extra_args: " $$CARGO_EXTRA_ARGS)
    message("lib: " $$RUST_BINDING_LIB)
    system(cd $$PWD/rust && cargo build $$CARGO_EXTRA_ARGS && cd ..)
} else {
    android {
        CARGO_BUILD_TYPE = aarch64-linux-android/release
        CARGO_EXTRA_ARGS = --release --target aarch64-linux-android
    } else {
        debug {
            CARGO_BUILD_TYPE = debug
            CARGO_EXTRA_ARGS =
        } else {
            CARGO_BUILD_TYPE = release
            CARGO_EXTRA_ARGS = --release
        }
    }

    RUST_BINDING_LIB = $$PWD/../target/$$CARGO_BUILD_TYPE/libcyberpixie_qml.a

    rust_binding.target = $$RUST_BINDING_LIB
    rust_binding.commands = cd $$PWD/rust && QMAKE="${QTDIR}/bin/qmake" TARGET_AR="llvm-ar" cargo build $$CARGO_EXTRA_ARGS && cd ..
    rust_bindings.depends = $$RUST_BINDING_FILES

    QMAKE_EXTRA_TARGETS += rust_binding
    PRE_TARGETDEPS += $$RUST_BINDING_LIB
}

linux: LIBS += -ldl
LIBS += $$RUST_BINDING_LIB

OTHER_FILES += $$RUST_BINDING_FILES

HEADERS += \
    filesreader.h
