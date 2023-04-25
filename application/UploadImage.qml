import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import Qt.labs.platform as Platform

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

        Platform.FileDialog {
            id: openImage

            folder: Platform.StandardPaths.writableLocation(Platform.StandardPaths.ImagesLocation)
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
                    const content = fileReader.readFile(openImage.file);
                    cyberpixie.uploadImage(content, rateInput.text * cyberpixie.stripLen);
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
