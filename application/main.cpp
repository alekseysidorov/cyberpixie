#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlFile>
#include <QFile>
#include <QQmlContext>

#include "filesreader.h"

extern "C" void register_cyberpixie_qml_types();

int main(int argc, char *argv[])
{
    register_cyberpixie_qml_types();

#if QT_VERSION < QT_VERSION_CHECK(6, 0, 0)
    QCoreApplication::setAttribute(Qt::AA_EnableHighDpiScaling);
#endif

    QGuiApplication app(argc, argv);

    FilesReader reader;

    QQmlApplicationEngine engine;
    const QUrl url(QStringLiteral("qrc:/main.qml"));
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);
    engine.rootContext()->setContextProperty("fileReader", &reader);
    engine.load(url);

    return app.exec();
}
