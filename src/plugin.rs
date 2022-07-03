use gst::glib;
use gst::gst_info;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst::BufferRef;
use gst_base::subclass::prelude::*;
use gst_video::subclass::prelude::*;
use gst_video::VideoFormat;
use gst_video::VideoFrameRef;
use image::ImageBuffer;
use image::Luma;

use std::i32;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::rects::rectanglify;
use crate::rects::Settings;

#[derive(Default)]
pub struct Rectanglify {
    settings: Mutex<Settings>,
}

static CAT: Lazy<gst::DebugCategory> = Lazy::new(|| {
    gst::DebugCategory::new(
        "rectanglify",
        gst::DebugColorFlags::empty(),
        Some("Rust rectangle transformation filter"),
    )
});

#[glib::object_subclass]
impl ObjectSubclass for Rectanglify {
    const NAME: &'static str = "Rectanglify";
    type Type = super::Rectanglify;
    type ParentType = gst_video::VideoFilter;
}

impl ObjectImpl for Rectanglify {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpecDouble::new(
                "rects-per-pixel",
                "Rectangles per black pixel",
                "The number of rectangles drawn for 1 black pixel's worth of darkness",
                0.0,
                f64::MAX,
                0.0001,
                glib::ParamFlags::READWRITE | gst::PARAM_FLAG_MUTABLE_PLAYING,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        obj: &Self::Type,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.name() {
            "rects-per-pixel" => {
                let mut settings = self.settings.lock().unwrap();
                let rects_per_pixel = value.get().expect("type checked upstream");
                gst_info!(
                    CAT,
                    obj: obj,
                    "Changing rects-per-pixel from {} to {}",
                    settings.rects_per_pixel,
                    rects_per_pixel
                );
                settings.rects_per_pixel = rects_per_pixel;
            }
            _ => unimplemented!(),
        }
    }

    // Called whenever a value of a property is read. It can be called
    // at any time from any thread.
    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "rects-per-pixel" => {
                let settings = self.settings.lock().unwrap();
                settings.rects_per_pixel.to_value()
            }
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for Rectanglify {}

fn caps() -> gst::Caps {
    gst::Caps::builder("video/x-raw")
        .field(
            "format",
            gst::List::new([
                // VideoFormat::Rgba.to_str(),
                // VideoFormat::Argb.to_str(),
                // VideoFormat::Bgra.to_str(),
                // VideoFormat::Abgr.to_str(),
                // VideoFormat::Rgb.to_str(),
                // VideoFormat::Bgr.to_str(),
                VideoFormat::Gray8.to_str(),
            ]),
        )
        .field("width", gst::IntRange::new(0, i32::MAX))
        .field("height", gst::IntRange::new(0, i32::MAX))
        .field(
            "framerate",
            gst::FractionRange::new(gst::Fraction::new(0, 1), gst::Fraction::new(i32::MAX, 1)),
        )
        .build()
}

impl ElementImpl for Rectanglify {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
            gst::subclass::ElementMetadata::new(
                "Rectanglify",
                "Filter/Effect/Converter/Video",
                env!("CARGO_PKG_DESCRIPTION"),
                env!("CARGO_PKG_AUTHORS"),
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
            // src pad capabilities
            let caps = caps();

            let src_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            vec![src_pad_template, sink_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }
}

impl BaseTransformImpl for Rectanglify {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::NeverInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;

    fn transform_caps(
        &self,
        _: &Self::Type,
        _: gst::PadDirection,
        _: &gst::Caps,
        _: Option<&gst::Caps>,
    ) -> Option<gst::Caps> {
        // the input and output are completely independent, we always support the full caps.
        Some(caps())
    }
}

impl VideoFilterImpl for Rectanglify {
    fn transform_frame(
        &self,
        _: &Self::Type,
        input: &VideoFrameRef<&BufferRef>,
        output: &mut VideoFrameRef<&mut BufferRef>,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let settings = *self.settings.lock().unwrap();

        // TODO: stuff other than pure grayscale
        let input: ImageBuffer<Luma<u8>, _> =
            ImageBuffer::from_raw(input.width(), input.height(), input.plane_data(0).unwrap())
                .unwrap();
        let mut output: ImageBuffer<Luma<u8>, _> = ImageBuffer::from_raw(
            output.width(),
            output.height(),
            output.plane_data_mut(0).unwrap(),
        )
        .unwrap();

        rectanglify(&input, &mut output, settings);

        Ok(gst::FlowSuccess::Ok)
    }
}
