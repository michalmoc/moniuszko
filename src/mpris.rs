use crate::commands::Command;
use crate::constants::{APP_NAME, FANCY_APP_NAME, MPRIS_NAME};
use crate::player::{PlaybackState, PlaybackStatus};
use async_channel::Sender;
use gtk4::glib::clone;
use zbus::{connection, interface};

enum PropertyChange {
    Status(PlaybackStatus),
}

struct Mpris {
    sender: Sender<Command>,
}

#[interface(name = "org.mpris.MediaPlayer2", spawn = false)]
impl Mpris {
    async fn raise(&self) {
        self.sender.send(Command::Raise).await.unwrap();
    }

    async fn quit(&self) {
        self.sender.send(Command::Quit).await.unwrap();
    }

    #[zbus(property)]
    async fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    async fn can_raise(&self) -> bool {
        // TODO
        false
    }

    #[zbus(property)]
    async fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    async fn identity(&self) -> &str {
        FANCY_APP_NAME
    }

    #[zbus(property)]
    async fn desktop_entry(&self) -> &str {
        APP_NAME
    }

    #[zbus(property)]
    async fn supported_uri_schemes(&self) -> &[&str] {
        &[]
    }

    #[zbus(property)]
    async fn supported_mime_types(&self) -> &[&str] {
        &[]
    }
}

struct MprisPlayer {
    sender: Sender<Command>,
    playback_status: PlaybackStatus,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    async fn next(&self) {
        self.sender.send(Command::Next).await.unwrap();
    }

    async fn previous(&self) {
        self.sender.send(Command::Previous).await.unwrap();
    }

    async fn pause(&self) {
        self.sender.send(Command::Pause).await.unwrap();
    }

    async fn play_pause(&self) {
        self.sender.send(Command::PlayPause).await.unwrap();
    }

    async fn stop(&self) {
        self.sender.send(Command::Stop).await.unwrap();
    }

    async fn play(&self) {
        self.sender.send(Command::Play).await.unwrap();
    }

    async fn seek(&self, offset: i64) {
        self.sender.send(Command::Seek(offset)).await.unwrap();
    }

    #[zbus(property)]
    async fn playback_status(&self) -> &str {
        self.playback_status.to_str()
    }

    // TODO: SetPosition, OpenUri, Seeked
}

pub async fn mpris(
    command_queue: Sender<Command>,
    playback_state: PlaybackState,
) -> anyhow::Result<()> {
    let connection = connection::Builder::session()?
        .name(MPRIS_NAME)?
        .serve_at(
            "/org/mpris/MediaPlayer2",
            Mpris {
                sender: command_queue.clone(),
            },
        )?
        .serve_at(
            "/org/mpris/MediaPlayer2",
            MprisPlayer {
                sender: command_queue,
                playback_status: PlaybackStatus::Stopped,
            },
        )?
        .build()
        .await?;

    let object_server = connection
        .object_server()
        .interface::<_, MprisPlayer>("/org/mpris/MediaPlayer2")
        .await?;

    let (sender, receiver) = async_channel::unbounded::<PropertyChange>();

    playback_state.connect_status_notify(clone!(
        #[strong]
        sender,
        move |p| {
            sender
                .send_blocking(PropertyChange::Status(p.status()))
                .unwrap();
        }
    ));

    loop {
        let change = receiver.recv().await?;
        match change {
            PropertyChange::Status(status) => {
                let mut os = object_server.get_mut().await;
                os.playback_status = status;
                os.playback_status_changed(object_server.signal_emitter())
                    .await?;
            }
        }
    }
}
