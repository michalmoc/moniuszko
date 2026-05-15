use const_format::concatcp;

pub const APP_NAME: &str = "moniuszko";
pub const FANCY_APP_NAME: &str = "Moniuszko";
pub const APP_ID: &str = concatcp!("com.example.", APP_NAME);
pub const MPRIS_NAME: &str = concatcp!("org.mpris.MediaPlayer2.", APP_NAME);
