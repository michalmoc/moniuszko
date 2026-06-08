use crate::constants::{APP_NAME, FANCY_APP_NAME};
use crate::control::commands::Command;
use async_channel::Sender;
use gettextrs::gettext;
use image::GenericImageView;
use ksni::TrayMethods;
use std::sync::LazyLock;

pub async fn run_tray(commands: Sender<Command>) {
    let tray = MyTray { commands };
    let _handle = tray.spawn().await.unwrap();
    std::future::pending::<()>().await
}

#[derive(Debug)]
struct MyTray {
    commands: Sender<Command>,
}

impl ksni::Tray for MyTray {
    fn id(&self) -> String {
        APP_NAME.into()
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        self.commands.send_blocking(Command::HideShow).unwrap()
    }

    fn title(&self) -> String {
        FANCY_APP_NAME.into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        static ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
            let img = image::load_from_memory_with_format(
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png")),
                image::ImageFormat::Png,
            )
            .expect("valid image");
            let (width, height) = img.dimensions();
            let mut data = img.into_rgba8().into_vec();
            assert_eq!(data.len() % 4, 0);
            for pixel in data.chunks_exact_mut(4) {
                pixel.rotate_right(1) // rgba to argb
            }
            ksni::Icon {
                width: width as i32,
                height: height as i32,
                data,
            }
        });

        vec![ICON.clone()]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: gettext("tray-hide-show").into(),
                activate: Box::new(|t: &mut MyTray| {
                    t.commands.send_blocking(Command::HideShow).unwrap()
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: gettext("tray-exit").into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|t: &mut MyTray| {
                    t.commands.send_blocking(Command::Quit).unwrap()
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
