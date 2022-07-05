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
use image::Pixel;
use image::Rgb;
use image::Rgba;

use std::i32;
use std::ops::Deref;
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
    use VideoFormat::*;
    gst::Caps::builder("video/x-raw")
        .field(
            "format",
            gst::List::new([Rgba.to_str(), Rgb.to_str(), Gray8.to_str()]),
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
        filter: Option<&gst::Caps>,
    ) -> Option<gst::Caps> {
        // the input and output are completely independent, we always support the full caps.
        let mut caps = caps();
        if let Some(filter) = filter {
            // we do have to apply any filters though
            caps = caps.intersect(filter)
        }
        Some(caps)
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

        // This stupid trait is needed because we can't make generic callbacks.
        trait FormatCb<C> {
            fn call(self, image: ImageBuffer<impl Pixel<Subpixel = u8>, C>);
        }

        fn with_image<C: Deref<Target = [u8]>>(
            width: u32,
            height: u32,
            format: VideoFormat,
            container: C,
            callback: impl FormatCb<C>,
        ) {
            macro_rules! formats {
                ($($gst:ident => $image:ty,)*) => {
                    match format {
                        $(
                        VideoFormat::$gst => {
                            let image = ImageBuffer::<$image, C>::from_raw(width, height, container).unwrap();
                            callback.call(image);
                        }
                        )*
                        _ => unimplemented!(),
                    }
                };
            }

            // TODO: more formats
            // see https://gstreamer.freedesktop.org/documentation/additional/design/mediatype-video-raw.html#formats
            formats! {
                Rgba => Rgba<u8>,
                Rgb => Rgb<u8>,
                Gray8 => Luma<u8>,
            }
        }

        with_image(
            input.width(),
            input.height(),
            input.format(),
            input.plane_data(0).unwrap(),
            (settings, output),
        );

        impl FormatCb<&[u8]> for (Settings, &mut VideoFrameRef<&mut BufferRef>) {
            fn call(self, input: ImageBuffer<impl Pixel<Subpixel = u8>, &[u8]>) {
                let (settings, output) = self;
                with_image(
                    output.width(),
                    output.height(),
                    output.format(),
                    output.plane_data_mut(0).unwrap(),
                    (settings, input),
                );
            }
        }

        impl<P: Pixel<Subpixel = u8>> FormatCb<&mut [u8]> for (Settings, ImageBuffer<P, &[u8]>) {
            fn call(self, mut output: ImageBuffer<impl Pixel<Subpixel = u8>, &mut [u8]>) {
                let (settings, input) = self;
                rectanglify(&input, &mut output, settings)
            }
        }

        Ok(gst::FlowSuccess::Ok)
    }
}
