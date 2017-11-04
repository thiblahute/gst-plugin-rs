// Copyright (C) 2016-2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_type = "cdylib"]

#[macro_use]
extern crate gst_plugin;
#[macro_use]
extern crate gstreamer as gst;
extern crate gstreamer_video as gst_video;

use gst_plugin::video_decoder::*;
use gst::prelude::*;

mod gifdec;

use gifdec::GifDec;

fn plugin_init(plugin: &gst::Plugin) -> bool {
    video_decoder_register(
        plugin,
        VideoDecoderInfo {
            name: "rsgifdec".into(),
            long_name: "Gif decoder".into(),
            description: "Decodes Gif Images".into(),
            classification: "Codec/Decoder/Video".into(),
            author: "Thibault Saunier <tsaunier@gnome.org>".into(),
            rank: 256 + 100,
            create_instance: GifDec::new_boxed,
            sinkcaps: gst::Caps::new_simple("image/gif", &[]),
            srccaps: gst::Caps::from_string("video/x-raw, framerate=25/1, format=ARGB").unwrap(),
        },
    );

    true
}

plugin_define!(
    b"rsgif\0",
    b"Rust GIF Plugin\0",
    plugin_init,
    b"1.0\0",
    b"MIT/X11\0",
    b"rsgif\0",
    b"rsgif\0",
    b"https://github.com/sdroege/rsplugin\0",
    b"2017-10-16\0"
);
