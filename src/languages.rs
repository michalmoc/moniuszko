use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, setlocale, textdomain};
use icu::collator::options::{CollatorOptions, Strength};
use icu::collator::{Collator, CollatorBorrowed};
use icu::locale::LanguageIdentifier;
use std::sync::OnceLock;
use sys_locale::get_locale;

pub fn set_global_locale_gettext() {
    let locale_dir =
        option_env!("LOCALE_DIR").unwrap_or(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/gettext"));

    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain("moniuszko", locale_dir).expect("Unable to bind the text domain");

    bind_textdomain_codeset("moniuszko", "UTF-8").expect("Unable to set text domain encoding");
    textdomain("moniuszko").expect("Unable to switch to the text domain");
}

pub static COLLATOR: OnceLock<CollatorBorrowed> = OnceLock::new();

pub fn init_collator() {
    let locale = get_locale().unwrap_or_else(|| String::from("en-US"));
    let locale = LanguageIdentifier::try_from_str(&locale).expect("Unable to parse the language");

    let mut options = CollatorOptions::default();
    options.strength = Some(Strength::Tertiary);

    COLLATOR
        .set(Collator::try_new(locale.into(), options).unwrap())
        .expect("Unable to set COLLATOR");
}
