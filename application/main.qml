import QtQuick
import QtCore
import QtQuick.Controls
import QtQuick.Window
import QtQuick.Dialogs
import Qt.labs.platform as Platform

import cyberpixie;

ApplicationWindow {
    id: app

    property bool deviceConnected

    width: 400
    height: 800
    visible: true
    title: qsTr("Tabs")

    SwipeView {
        id: swipeView
        anchors.fill: parent
        currentIndex: tabBar.currentIndex
        enabled: !cyberpixie.busy

        DeviceInfo {
        }

        UploadImage {
        }
    }

    Balloon {
        id: balloon

        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: 15
            topMargin: 100
        }
    }

    footer: TabBar {
        id: tabBar
        currentIndex: swipeView.currentIndex

        TabButton {
            text: qsTr("Device information")
        }
        TabButton {
            text: qsTr("Upload image")
        }
    }

    DeviceHandle {
        id: cyberpixie

        function nextImage() {
            let next = (cyberpixie.currentImage + 1) % (cyberpixie.imagesCount)
            cyberpixie.setImage(next)
        }

        function prevImage() {
            let prev = (cyberpixie.currentImage + cyberpixie.imagesCount + 1) % (cyberpixie.imagesCount)
            cyberpixie.setImage(prev)
        }

        onError: function(message) {
            balloon.show("An error occurred", message)
            app.deviceConnected = false
        }

        onImageUploaded: {
            balloon.show("Image uploaded", "")
        }

        onStripLenChanged: {
            if (stripLen > 0) {
                app.deviceConnected = true
            }
        }
    }
}
