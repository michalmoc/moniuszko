use crate::commands::Command;
use crate::constants::{APP_NAME, FANCY_APP_NAME, MPRIS_NAME};
use async_channel::Sender;
use zbus::{connection, interface};

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
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    async fn next(&self) {
        self.sender.send(Command::Next).await.unwrap();
    }
}

pub async fn mpris(command_queue: Sender<Command>) -> anyhow::Result<()> {
    let _connection = connection::Builder::session()?
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
            },
        )?
        .build()
        .await?;

    loop {
        std::future::pending::<()>().await;
    }
}
