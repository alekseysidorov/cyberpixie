import QtQuick 2.12
import QtQuick.Controls 2.5
import QtQuick.Layouts 1.12

Page {
    width: 600
    height: 400

    header: Label {
        text: qsTr("Device information page")
        font.pixelSize: Qt.application.font.pixelSize * 2
        padding: 10
    }

    ColumnLayout {
        anchors {
            fill: parent
            margins: 10
        }

        Label {
            Layout.alignment: Qt.AlignHCenter

            visible: !app.deviceConnected
            text: "No information about the device"
        }

        Label {
            Layout.alignment: Qt.AlignHCenter

            visible: app.deviceConnected
            text: qsTr("Strip length: %1").arg(cyberpixie.stripLen)
        }

        Label {
            Layout.alignment: Qt.AlignHCenter

            visible: app.deviceConnected
            text: qsTr("Images count: %1").arg(cyberpixie.imagesCount)
        }

        Label {
            Layout.alignment: Qt.AlignHCenter

            visible: app.deviceConnected
            text: qsTr("Current image: %1").arg(cyberpixie.currentImage)
        }

        Button {
            Layout.alignment: Qt.AlignHCenter

            visible: app.deviceConnected

            text: qsTr("Clear all images")

            onClicked: cyberpixie.clearImages()
        }

        Item {
            width: 1
            Layout.fillHeight: true
        }

        RowLayout {
            visible: app.deviceConnected

            Layout.alignment: Qt.AlignHCenter

            Button {
                text: qsTr("Prev image")

                onClicked: cyberpixie.prevImage()
            }

            Button {
                text: qsTr("Next image")

                onClicked: cyberpixie.nextImage()
            }
        }

        Button {
            visible: !app.deviceConnected

            Layout.alignment: Qt.AlignHCenter

            text: qsTr("Connect")

            onClicked: {
                cyberpixie.deviceInfo()
            }
        }
    }
}
