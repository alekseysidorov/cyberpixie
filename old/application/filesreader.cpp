#include "filesreader.h"

#include <QQmlFile>
#include <QFile>

FilesReader::FilesReader(QObject *parent)
    : QObject{parent}
{

}

QByteArray FilesReader::readFile(QUrl url) const {
    QFile file(QQmlFile::urlToLocalFileOrQrc(url));
    if (!file.open(QFile::ReadOnly)) {
        emit error(tr("Could not open file. '%1'").arg(file.errorString()));
        return QByteArray();
    }

    QByteArray data = file.readAll();
    return data;
}
