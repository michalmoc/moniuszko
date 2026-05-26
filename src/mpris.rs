use crate::commands::Command;
use crate::constants::{APP_NAME, FANCY_APP_NAME, MPRIS_NAME};
use crate::database::DatabasePtr;
use crate::player::{PlaybackState, PlaybackStatus, RepeatMode};
use async_channel::Sender;
use gtk4::glib::clone;
use itertools::Itertools;
use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{connection, interface};

enum PropertyChange {
    Status(PlaybackStatus),
    RepeatMode(RepeatMode),
    Current,
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
        true
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
    loop_status: String,
    shuffle: bool,
    metadata: HashMap<String, OwnedValue>,
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

    #[zbus(property)]
    async fn loop_status(&self) -> &str {
        &self.loop_status
    }

    #[zbus(property)]
    async fn shuffle(&self) -> bool {
        self.shuffle
    }

    #[zbus(property)]
    async fn metadata(&self) -> HashMap<String, OwnedValue> {
        self.metadata.clone()
    }

    #[zbus(property)]
    async fn can_go_next(&self) -> bool {
        true
    }
    #[zbus(property)]
    async fn can_go_previous(&self) -> bool {
        true
    }
    #[zbus(property)]
    async fn can_play(&self) -> bool {
        true
    }
    #[zbus(property)]
    async fn can_pause(&self) -> bool {
        true
    }
    #[zbus(property)]
    async fn can_seek(&self) -> bool {
        true
    }
    #[zbus(property)]
    async fn can_control(&self) -> bool {
        true
    }

    // TODO: SetPosition, OpenUri, Seeked, Rate, Volume, Position, MinimumRate, MaximumRate
}

pub async fn mpris(
    command_queue: Sender<Command>,
    playback_state: PlaybackState,
    database: DatabasePtr,
) -> anyhow::Result<()> {
    let connection = connection::Builder::session()?
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
                loop_status: "None".to_string(),
                shuffle: false,
                metadata: HashMap::new(),
            },
        )?
        .name(MPRIS_NAME)?
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

    playback_state.connect_repeat_mode_notify(clone!(
        #[strong]
        sender,
        move |p| {
            sender
                .send_blocking(PropertyChange::RepeatMode(p.repeat_mode()))
                .unwrap();
        }
    ));

    playback_state.connect_current_notify(clone!(
        #[strong]
        sender,
        move |_| {
            sender.send_blocking(PropertyChange::Current).unwrap();
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
            PropertyChange::RepeatMode(rm) => {
                let mut os = object_server.get_mut().await;
                match rm {
                    RepeatMode::Single => {
                        os.loop_status = "Track".to_string();
                        os.shuffle = false;
                    }
                    RepeatMode::All => {
                        os.loop_status = "Playlist".to_string();
                        os.shuffle = false;
                    }
                    RepeatMode::Shuffle => {
                        os.loop_status = "Playlist".to_string();
                        os.shuffle = true;
                    }
                }
                os.loop_status_changed(object_server.signal_emitter())
                    .await?;
                os.shuffle_changed(object_server.signal_emitter()).await?;
            }
            PropertyChange::Current => {
                if let Some(current) = playback_state.current() {
                    let mut metadata = HashMap::new();

                    metadata.insert(
                        "mpris:trackid".to_string(),
                        Value::new(current.uuid().to_string()).try_into_owned()?,
                    );

                    let db = database.read().unwrap();
                    let current = &db[current.stored_track()];

                    metadata.insert(
                        "mpris:length".to_string(),
                        Value::new(current.duration.as_micros() as i64).try_into_owned()?,
                    );

                    metadata.insert(
                        "xesam:title".to_string(),
                        Value::new(current.title.to_string()).try_into_owned()?,
                    );

                    let artists = current
                        .artist_ids
                        .iter()
                        .filter_map(|a| db[*a].name)
                        .map(|a| a.to_string())
                        .collect_vec();

                    metadata.insert(
                        "xesam:artist".to_string(),
                        Value::new(artists).try_into_owned()?,
                    );

                    let album = &db[current.album];
                    metadata.insert(
                        "xesam:album".to_string(),
                        Value::new(album.title.to_string()).try_into_owned()?,
                    );

                    metadata.insert(
                        "mpris:artUrl".to_string(),
                        Value::new("file://".to_owned() + &album.cover).try_into_owned()?,
                    );

                    let mut os = object_server.get_mut().await;
                    os.metadata = metadata;
                    os.metadata_changed(object_server.signal_emitter()).await?;
                    //TODO more
                } else {
                    let mut os = object_server.get_mut().await;
                    os.metadata = HashMap::new();
                    os.metadata_changed(object_server.signal_emitter()).await?;
                }
            }
        }
    }
}
