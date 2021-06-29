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
            anchors.horizontalCenter: parent.horizontalCenter
            visible: !app.deviceConnected
            text: "No information about the device"
        }

        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            visible: app.deviceConnected
            text: qsTr("Strip length: %1", cyberpixie.stripLen)
        }

        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            visible: app.deviceConnected
            text: qsTr("Images count: %1", cyberpixie.imagesCount)
        }

        RowLayout {
            anchors.horizontalCenter: parent.horizontalCenter
            visible: app.deviceConnected

            Button {
                text: qsTr("Prev")
            }

            Button {
                text: qsTr("Next")
            }
        }

        Item {
            width: 1
            Layout.fillHeight: true
        }

        Button {
            anchors.horizontalCenter: parent.horizontalCenter
            text: qsTr("Connect")

            onClicked: {
                cyberpixie.deviceInfo()
            }
        }
    }
}
