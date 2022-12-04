import QtQuick 2.12
import QtQuick.Controls 2.5
import QtQuick.Layouts 1.12
import QtQuick.Dialogs 1.2
import Qt.labs.platform 1.1

Page {
    header: Label {
        text: qsTr("Upload image")
        font.pixelSize: Qt.application.font.pixelSize * 2
        padding: 10
    }

    ColumnLayout {
        anchors {
            fill: parent
            margins: 10
        }

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.fillHeight: true
            Layout.fillWidth: true

            border.color: "black"
            color: "#555"

            Image {
                id: image

                anchors {
                    fill: parent
                    margins: 6
                }

                source: openImage.file
                fillMode: Image.PreserveAspectFit
                smooth: true
            }
        }

        FileDialog {
            id: openImage

            folder: StandardPaths.writableLocation(StandardPaths.ImagesLocation)
            nameFilters: ["Images (*.png *jpg *jpeg)"]
            file: ""
        }

        Label {
            text: "Image refresh rate (hz):"

            Layout.alignment: Qt.AlignHCenter
        }

        TextField {
            id: rateInput

            Layout.alignment: Qt.AlignHCenter

            placeholderText: "hz"
            validator: IntValidator { bottom: 10; top: 50; }
            text: "25"
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 10

            Button {
                text: qsTr("Select image")

                onClicked: {
                    openImage.open()
                }
            }

            Button {
                enabled: app.deviceConnected

                text: qsTr("Upload")

                onClicked: {
                    cyberpixie.uploadImage(openImage.file.toString(), rateInput.text * cyberpixie.stripLen);
                }

            }
        }
    }
}

/*##^##
Designer {
    D{i:0;autoSize:true;height:480;width:640}
}
##^##*/
