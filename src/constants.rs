use const_format::concatcp;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_ID: &str = concatcp!("com.example.", APP_NAME);
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
