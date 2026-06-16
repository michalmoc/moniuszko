use crate::data::playback_status::PlaybackStatus;
use gtk4::glib;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use log::{info, warn};
use std::time::Duration;

mod imp {
    use crate::data::playback_status::PlaybackStatus;
    use crate::data::repeat_mode::RepeatMode;
    use crate::ui::playlist_item::PlaylistItem;
    use adw::glib::ControlFlow;
    use adw::glib::subclass::Signal;
    use async_channel::{Receiver, Sender};
    use gtk4::glib;
    use gtk4::glib::{Object, Properties, clone};
    use gtk4::prelude::ObjectExt;
    use gtk4::subclass::prelude::{
        DerivedObjectProperties, ObjectImplExt, ObjectSubclassExt, ObjectSubclassIsExt,
    };
    use gtk4::subclass::prelude::{ObjectImpl, ObjectSubclass};
    use log::{info, warn};
    use rodio::source::{EmptyCallback, SineWave};
    use rodio::{Decoder, MixerDeviceSink, Player, Source};
    use std::cell::{Cell, RefCell};
    use std::fs::File;
    use std::sync::OnceLock;
    use std::time::Duration;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PlaybackState)]
    pub struct PlaybackState {
        #[property(name="volume", get = Self::get_volume, set = Self::set_volume, type = f32)]
        #[property(name="progress", get = Self::get_progress, type = i64)]
        pub player: RefCell<Player>,

        #[property(get=Self::get_playing, set=Self::set_playing)]
        pub playing: Cell<bool>,

        #[property(get)]
        pub duration: Cell<i64>,

        #[property(get, set, default)]
        pub repeat_mode: Cell<RepeatMode>,

        #[property(get, set, default)]
        pub status: Cell<PlaybackStatus>,

        #[property(get, set=Self::change_current, nullable)]
        pub current: RefCell<Option<PlaylistItem>>,

        pub sink: RefCell<MixerDeviceSink>,
        sender: Sender<EndedMsg>,
        receiver: Receiver<EndedMsg>,
    }

    struct EndedMsg;

    impl Default for PlaybackState {
        fn default() -> PlaybackState {
            let mut sink =
                rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
            sink.log_on_drop(true);

            let player = Player::connect_new(&sink.mixer());

            let (sender, receiver) = async_channel::unbounded::<EndedMsg>();

            PlaybackState {
                player: RefCell::new(player),
                playing: Cell::new(false),
                duration: Cell::new(0),
                repeat_mode: Cell::new(RepeatMode::All),
                status: Cell::new(PlaybackStatus::Stopped),
                current: RefCell::new(None),
                sink: RefCell::new(sink),
                sender,
                receiver,
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaybackState {
        const NAME: &'static str = "PlaybackState";
        type Type = super::PlaybackState;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaybackState {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            glib::spawn_future_local(clone!(
                #[weak]
                obj,
                async move {
                    while let Ok(_) = obj.imp().receiver.recv().await {
                        obj.emit_by_name::<()>("ended", &[]);
                    }
                }
            ));

            let obj_clone = obj.clone();
            glib::timeout_add_local(Duration::from_millis(900), move || {
                if obj_clone.playing() {
                    obj_clone.notify_progress()
                }
                ControlFlow::Continue
            });
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("ended").build()])
        }
    }

    impl PlaybackState {
        fn change_current(&self, current: Option<PlaylistItem>) {
            if let Some(old) = self.current.replace(current) {
                old.set_is_playing(false);
            }

            self.player.borrow().stop();

            if let Some(current) = self.current.borrow().as_ref() {
                current.set_is_playing(true);

                match File::open(current.path()) {
                    Err(e) => {
                        warn!("Failed to open file: {:?}", e);
                    }
                    Ok(file) => match Decoder::try_from(file) {
                        Err(e) => {
                            warn!("Failed to decode file: {:?}", e);
                        }
                        Ok(decoder) => {
                            if let Some(duration) = decoder.total_duration() {
                                self.duration.set(duration.as_millis() as i64);
                                self.obj().notify_duration();
                            }

                            self.player.borrow().append(decoder);
                            self.player
                                .borrow()
                                .append(EmptyCallback::new(Box::new(clone!(
                                    #[strong(rename_to=sender)]
                                    self.sender,
                                    move || sender.send_blocking(EndedMsg).unwrap()
                                ))));
                            self.player.borrow().append(rodio::source::Empty::new());
                        }
                    },
                }
            }

            self.obj().compute_status();
        }

        fn get_volume(&self) -> f32 {
            self.player.borrow().volume()
        }

        fn set_volume(&self, val: f32) {
            self.player.borrow().set_volume(val);
        }

        fn get_progress(&self) -> i64 {
            self.player.borrow().get_pos().as_millis() as i64
        }

        fn get_playing(&self) -> bool {
            let player = self.player.borrow();
            !(player.empty() || player.is_paused())
        }

        fn set_playing(&self, val: bool) {
            {
                let player = self.player.borrow();
                if val { player.play() } else { player.pause() }
            }

            self.playing.set(val);
            self.obj().compute_status();
        }
    }
}

glib::wrapper! {
    pub struct PlaybackState(ObjectSubclass<imp::PlaybackState>);
}

impl PlaybackState {
    pub fn seek(&self, progress: i64) {
        if let Err(err) = self
            .imp()
            .player
            .borrow()
            .try_seek(Duration::from_millis(progress as u64))
        {
            warn!("Failed to seek: {:?}", err);
        }
    }

    fn compute_status(&self) {
        if self.current().is_none() || self.imp().player.borrow().empty() {
            if self.status() != PlaybackStatus::Stopped {
                self.set_status(PlaybackStatus::Stopped);
            }
        } else {
            if self.imp().player.borrow().is_paused() {
                if self.status() != PlaybackStatus::Paused {
                    self.set_status(PlaybackStatus::Paused);
                }
            } else {
                if self.status() != PlaybackStatus::Playing {
                    self.set_status(PlaybackStatus::Playing);
                }
            }
        }
    }
}
