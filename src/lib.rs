use gst::glib;
use gst::prelude::*;

mod plugin;
mod rects;

glib::wrapper! {
    pub struct Rectanglify(ObjectSubclass<plugin::Rectanglify>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

fn plugin_init(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "rectanglify",
        gst::Rank::None,
        Rectanglify::static_type(),
    )
}

gst::plugin_define!(
    hsv,
    env!("CARGO_PKG_DESCRIPTION"),
    plugin_init,
    concat!(env!("CARGO_PKG_VERSION"), "-", env!("COMMIT_ID")),
    "MIT/X11",
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_REPOSITORY"),
    env!("BUILD_REL_DATE")
);
