import QtQuick 2.12
import QtQuick.Controls 2.5
import QtQuick.Layouts 1.12
import QtQuick.Dialogs 1.2
import Qt.labs.platform 1.1

Page {
    width: 600
    height: 400

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

        Image {
            id: image

            Layout.alignment: Qt.AlignHCenter
            Layout.fillHeight: true

            width: parent.width
            source: openImage.file

            fillMode: Image.PreserveAspectFit
            smooth: true
        }

        FileDialog {
            id: openImage

            property bool selected: false

            folder: StandardPaths.writableLocation(StandardPaths.ImagesLocation)
            nameFilters: ["Images (*.png *jpg *jpeg)"]
            file: ""
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter

            Button {
                visible: app.deviceConnected && !openImage.selected

                text: qsTr("Select image")

                onClicked: {
                    openImage.open()
                    openImage.selected = true
                }
            }

            Button {
                visible: app.deviceConnected && openImage.selected

                text: qsTr("Upload")

                onClicked: {
                    cyberpixie.uploadImage(openImage.file.toString(), 25 * 48);
                    openImage.selected = false
                    openImage.file = ""
                }
            }

            Button {
                visible: app.deviceConnected

                text: qsTr("Clear images")

                onClicked: cyberpixie.clearImages()
            }
        }
    }
}
