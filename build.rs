fn main() {
    // TODO: remove
    fluent_zero_build::generate_static_cache("assets/locales");

    glib_build_tools::compile_resources(
        &["src/resources"],
        "src/resources/resources.gresource.xml",
        "moniuszko.gresource",
    );
}
