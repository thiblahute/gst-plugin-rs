// Copyright (C) 2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::sync::Mutex;

use object::*;
use element::*;
use error::*;

use glib;
use gst;
use gst::prelude::*;

use gst_video;
use gst_video::prelude::*;

use object::*;
use element::*;
use base_video_decoder::*;

pub use base_video_decoder::VideoDecoder;

pub trait VideoDecImpl: Send + 'static {
    fn start(&mut self, decoder: &VideoDecoder) -> Result<(), ErrorMessage>;
    fn stop(&mut self, decoder: &VideoDecoder) -> Result<(), ErrorMessage>;
    fn finish(&mut self, decoder: &VideoDecoder) -> Result<(), FlowError>;
    fn set_format(&mut self, decoder: &VideoDecoder, state: &gst_video::VideoCodecState) -> bool;
    fn handle_frame(&mut self, decoder: &VideoDecoder, frame: &gst_video::VideoCodecFrame)
        -> Result<(), FlowError>;
}

struct VideoDec {
    cat: gst::DebugCategory,
    imp: Mutex<Box<VideoDecImpl>>,
}

pub struct VideoDecoderInfo {
    pub name: String,
    pub long_name: String,
    pub description: String,
    pub classification: String,
    pub author: String,
    pub rank: u32,
    pub create_instance: fn(&VideoDecoder) -> Box<VideoDecImpl>,
    pub sinkcaps: gst::Caps,
    pub srccaps: gst::Caps,
}

impl VideoDec {
    fn new(videodecoder: &VideoDecoder, videodecoder_info: &VideoDecoderInfo) -> Self {
        let videodecoder_impl = (videodecoder_info.create_instance)(videodecoder);

        Self {
            cat: gst::DebugCategory::new(
                "VideoDecoder",
                gst::DebugColorFlags::empty(),
                "Rust video decoder",
            ),
            imp: Mutex::new(videodecoder_impl),
        }
    }

    fn class_init(klass: &mut VideoDecoderClass, videodecoder_info: &VideoDecoderInfo) {
        klass.set_metadata(
            &videodecoder_info.long_name,
            &videodecoder_info.classification,
            &videodecoder_info.description,
            &videodecoder_info.author,
        );

        let pad_template = gst::PadTemplate::new(
            "sink",
            gst::PadDirection::Sink,
            gst::PadPresence::Always,
            &videodecoder_info.sinkcaps,
        );
        klass.add_pad_template(pad_template);

        let pad_template = gst::PadTemplate::new(
            "src",
            gst::PadDirection::Src,
            gst::PadPresence::Always,
            &videodecoder_info.srccaps,
        );
        klass.add_pad_template(pad_template);

    }

    fn init(element: &VideoDecoder, videodecoder_info: &VideoDecoderInfo) ->
            Box<VideoDecoderImpl<VideoDecoder>> {
        let imp = Self::new(element, videodecoder_info);
        Box::new(imp)
    }
}

impl ObjectImpl<VideoDecoder> for VideoDec {}
impl ElementImpl<VideoDecoder> for VideoDec {}
impl VideoDecoderImpl<VideoDecoder> for VideoDec {
    fn start(&self, decoder: &VideoDecoder) -> bool {
        let videodecoder_impl = &mut self.imp.lock().unwrap();
        match videodecoder_impl.start(decoder) {
            Ok(..) => {
                gst_trace!(self.cat, obj: decoder, "Started successfully");
                true
            }
            Err(ref msg) => {
                gst_error!(self.cat, obj: decoder, "Failed to start: {:?}", msg);
                msg.post(decoder);
                false
            }
        }
    }

    fn finish(&self, decoder: &VideoDecoder) -> gst::FlowReturn {
        let videodecoder_impl = &mut self.imp.lock().unwrap();
        match videodecoder_impl.finish(decoder) {
            Ok(()) => gst::FlowReturn::Ok,
            Err(flow_error) => {
                gst_error!(self.cat, obj: decoder, "Failed to finish decoder: {:?}", flow_error);
                match flow_error {
                    FlowError::NotNegotiated(ref msg) | FlowError::Error(ref msg) => {
                        msg.post(decoder);
                    }
                    _ => (),
                }
                flow_error.to_native()
            }
        }
    }

    fn stop(&self, decoder: &VideoDecoder) -> bool {
        let videodecoder_impl = &mut self.imp.lock().unwrap();
        match videodecoder_impl.stop(decoder) {
            Ok(..) => {
                gst_trace!(self.cat, obj: decoder, "Stoped successfully");
                true
            }
            Err(ref msg) => {
                gst_error!(self.cat, obj: decoder, "Failed to stop: {:?}", msg);
                msg.post(decoder);
                false
            }
        }
    }

    fn set_format(&self, decoder: &VideoDecoder, state: &gst_video::VideoCodecState) -> bool {
        let videodecoder_impl = &mut self.imp.lock().unwrap();
        videodecoder_impl.set_format(decoder, state)
    }

    fn handle_frame(
        &self,
        decoder: &VideoDecoder,
        frame: &gst_video::VideoCodecFrame
    ) -> gst::FlowReturn {
        let videodecoder_impl = &mut self.imp.lock().unwrap();
        match videodecoder_impl.handle_frame(decoder, frame) {
            Ok(()) => gst::FlowReturn::Ok,
            Err(flow_error) => {
                gst_error!(self.cat, obj: decoder, "Failed to handle frame: {:?}", flow_error);
                match flow_error {
                    FlowError::NotNegotiated(ref msg) | FlowError::Error(ref msg) => {
                        msg.post(decoder);
                    }
                    _ => (),
                }
                flow_error.to_native()
            }
        }
    }
}

struct VideoDecoderStatic {
    name: String,
    videodecoder_info: VideoDecoderInfo,
}

impl ImplTypeStatic<VideoDecoder> for VideoDecoderStatic {
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    fn new(&self, element: &VideoDecoder) -> Box<VideoDecoderImpl<VideoDecoder>> {
        VideoDec::init(element, &self.videodecoder_info)
    }

    fn class_init(&self, klass: &mut VideoDecoderClass) {
        VideoDec::class_init(klass, &self.videodecoder_info);
    }

    fn type_init(&self, token: &TypeInitToken, type_: glib::Type) {}
}

pub fn video_decoder_register(plugin: &gst::Plugin, videodecoder_info: VideoDecoderInfo) {
    let name = videodecoder_info.name.clone();
    let rank = videodecoder_info.rank;

    let videodecoder_static = VideoDecoderStatic {
        name: format!("VideoDec-{}", name),
        videodecoder_info: videodecoder_info,
    };

    let type_ = register_type(videodecoder_static);
    gst::Element::register(plugin, &name, rank, type_);
}
