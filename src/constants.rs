use const_format::concatcp;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_ID: &str = concatcp!("com.example.", APP_NAME);
