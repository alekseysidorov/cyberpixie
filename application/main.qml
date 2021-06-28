import QtQuick 2.12
import QtQuick.Controls 2.5

import cyberpixie 1.0

ApplicationWindow {
    width: 640
    height: 480
    visible: true
    title: qsTr("Tabs")

    SwipeView {
        id: swipeView
        anchors.fill: parent
        currentIndex: tabBar.currentIndex

        DeviceInfo {
        }

        UploadImage {
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
    }
}
