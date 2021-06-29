import QtQuick 2.0
import QtQuick.Layouts 1.0
import QtQuick.Controls 2.0

Rectangle {
    id: balloon

    property alias title: title.text
    property alias body: body.text

    function show(title, body) {
        balloon.title = title;
        balloon.body = body;
        balloon.opacity = 1;
        timer.start()
    }

    width: 200
    height: childrenRect.height
    opacity: 0

    z: 100500
    radius: 10
    color: "transparent"
    smooth: true

    Rectangle {
        anchors.fill: parent
        color: "black"
        radius: balloon.radius
        opacity: 0.70
    }

    Behavior on opacity {
        NumberAnimation {
            easing.type: Easing.InOutQuad;
        }
    }

    ColumnLayout {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.margins: 12

        spacing: 12

        Label {
            id: title

            Layout.fillWidth: true

            color: "white"
            horizontalAlignment: Text.AlignHCenter
            font.pointSize: 16
            font.bold: true
            elide: Text.ElideRight
        }

        Label {
            id: body

            Layout.fillWidth: true

            color: "white"
            horizontalAlignment: Text.AlignHCenter
            text: "Message body fdvfdvvfdvfds sdcsd dscsd sdcdddsd asdsasdsadsdas asdas fvfdvdff"
            font.pointSize: 14
            wrapMode: Text.WordWrap
        }
    }

    Timer {
        id: timer

        interval: 2000
        running: false
        repeat: false

        onTriggered: {
            balloon.opacity = 0
        }
    }
}
