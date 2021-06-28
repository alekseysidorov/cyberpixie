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
            horizontalCenter: parent.horizontalCenter
            top: parent.top
            bottom: parent.bottom
            margins: 10
        }

        Label {
            text: "No information"
        }

        Item {
            width: 1
            Layout.fillHeight: true
        }

        Button {
            text: qsTr("Connect")

            onClicked: {
                console.log(cyberpixie.compute_greetings("Hello, "))
            }
        }
    }
}
