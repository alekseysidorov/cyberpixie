#ifndef FILESREADER_H
#define FILESREADER_H

#include <QtQml>

class FilesReader : public QObject
{
    Q_OBJECT
    QML_ELEMENT
public:
    explicit FilesReader(QObject *parent = nullptr);
    Q_INVOKABLE QByteArray readFile(QUrl url) const;
signals:
    void error(QString) const;
};

#endif // FILESREADER_H
