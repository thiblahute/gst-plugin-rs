// Copyright (C) 2016-2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use gst_plugin::error::*;
use gst_plugin::adapter::*;
use gst_plugin::video_decoder::*;

use std;
use std::io;
use std::sync::{Arc, Mutex};

use gst;

use gst_video;
use gst_video::VideoDecoderExt;

extern crate gif;
use self::gif::SetParameter;

pub struct GifDec {
    cat: gst::DebugCategory,
    reader: Option<gif::Reader<RcAdapter>>,
    adapter: RcAdapter,
    in_state: Option<gst_video::VideoCodecState>,
    background: Option<gst::Buffer>,
    current_start: u64,
}

#[derive(Clone)]
struct RcAdapter(Arc<Mutex<Adapter>>);

impl RcAdapter {
    fn new() -> RcAdapter {
        RcAdapter(Arc::new(Mutex::new(Adapter::new())))
    }

    pub fn push(&mut self, buffer: gst::Buffer) {
        self.0.lock().unwrap().push(buffer)
    }
}

impl io::Read for RcAdapter {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.0.lock().unwrap().read(buf)
    }
}

impl GifDec {
    pub fn new(_decoder: &VideoDecoder) -> GifDec {
        GifDec {
            cat: gst::DebugCategory::new(
                "rsgifdec",
                gst::DebugColorFlags::empty(),
                "Rust gif decoder",
            ),
            reader: None,
            adapter: RcAdapter::new(),
            in_state: None,
            background: None,
            current_start: 0,
        }
    }

    fn process_frames(&mut self, decoder: &VideoDecoder) -> Result<(), FlowError> {
        let istate = std::mem::replace(&mut self.in_state, None).unwrap();
        let mut reader = std::mem::replace(&mut self.reader, None).unwrap();
        let mut i = 0;
        let mut start = self.current_start;

        if decoder.get_output_state().is_none() {
            let swidth = reader.width();
            let sheight = reader.height();
            decoder.set_output_state(
                gst_video::VideoFormat::Rgba,
                swidth as u32,
                sheight as u32,
                Some(&istate.clone()),
            );
            let info = decoder.get_output_state().unwrap().info();
            let mut data = Vec::with_capacity(info.size());
            for i in 0..info.size() {
                data.push(0)
            }
            self.background = Some(gst::Buffer::from_slice(data).unwrap());
        }

        let mut background = std::mem::replace(&mut self.background, None).unwrap();
        let mut buffer = gst::Buffer::new();
        let mut res = Ok(());
        loop {
            match reader.read_next_frame() {
                Ok(f) => {
                    match f {
                        Some(frame) => {
                            let vinfo = &decoder.get_output_state().unwrap().info();
                            let gstframe = decoder.get_frame(i).unwrap();
                            let decoded_buf =
                                gst::Buffer::from_slice(frame.buffer.to_vec()).unwrap();

                            let mut outmap = gst_video::VideoFrame::from_buffer_writable(
                                background.copy_deep(),
                                &vinfo,
                            ).unwrap();

                            // OPTIMIZEME!
                            {
                                let stride = outmap.plane_stride()[0] as usize;
                                let mut y = 0;
                                let mut inpix = 0;
                                let inbuf = &frame.buffer;
                                let top = frame.top;
                                let left = frame.left;

                                let plane = outmap.plane_data_mut(0).unwrap();
                                for line in plane.chunks_mut(stride) {
                                    let mut x = 0;
                                    for rgba in line.chunks_mut(4) {
                                        if !(y < top || x < left || x > (left + frame.width - 1)
                                            || y > (top + frame.height - 1))
                                        {
                                            if inbuf[inpix + 3] != 0 {
                                                rgba[0] = inbuf[inpix];
                                                rgba[1] = inbuf[inpix + 1];
                                                rgba[2] = inbuf[inpix + 2];
                                                rgba[3] = inbuf[inpix + 3];
                                            }

                                            inpix += 4;
                                        }

                                        x += 1;
                                    }
                                    y += 1;
                                }
                            }
                            buffer = outmap.into_buffer();

                            background = buffer.copy_deep();
                            let mut wbuf = buffer.get_mut().unwrap();
                            wbuf.set_pts(gst::ClockTime::from_nseconds(start));
                            start += gst::ClockTime::from_mseconds(frame.delay as u64)
                                .nanoseconds()
                                .unwrap();
                            gstframe.set_output_buffer(&wbuf);
                            decoder.finish_frame(&gstframe);

                            i += 1;
                        }
                        None => {
                            println!("No more!");
                            break;
                        }
                    }
                }
                Err(err) => {
                    match err {
                        gif::DecodingError::Io(ref err) => {
                            println!("Not enough data! {:?}", err);
                            break;
                        }
                        _ => {
                            res = Err(FlowError::Error(error_msg!(
                                        gst::LibraryError::Failed,
                                        ["Gif decoder failed: {:?}", err]
                                        )));
                        }
                    }

                    break;
                }
            }
        }

        std::mem::replace(&mut self.reader, Some(reader));
        std::mem::replace(&mut self.in_state, Some(istate));
        std::mem::replace(&mut self.background, Some(background));
        self.current_start = start;

        res
    }

    pub fn new_boxed(decoder: &VideoDecoder) -> Box<VideoDecImpl> {
        Box::new(Self::new(decoder))
    }
}

impl VideoDecImpl for GifDec {
    fn start(&mut self, _decoder: &VideoDecoder) -> Result<(), ErrorMessage> {
        self.current_start = 0;
        Ok(())
    }

    fn stop(&mut self, _decoder: &VideoDecoder) -> Result<(), ErrorMessage> {
        Ok(())
    }

    fn finish(&mut self, decoder: &VideoDecoder) -> Result<(), FlowError> {
        self.process_frames(decoder)
    }

    fn set_format(
        &mut self,
        _decoder: &VideoDecoder,
        state: &gst_video::VideoCodecState,
    ) -> bool {
        println!("Set format from: {}", state.caps().to_string());

        std::mem::replace(&mut self.in_state, Some(state.clone()));
        true
    }

    fn handle_frame(
        &mut self,
        decoder: &VideoDecoder,
        frame: &gst_video::VideoCodecFrame,
    ) -> Result<(), FlowError> {
        if self.reader.is_none() {
            {
                self.adapter.push(frame.input_buffer().copy_deep());
            }

            println!("Here we go..");
            let mut decoder = gif::Decoder::new(self.adapter.clone());
            decoder.set(gif::ColorOutput::RGBA);
            match decoder.read_info() {
                Ok(reader) => {
                    self.reader = Some(reader);
                }
                Err(_) => {
                    return Ok(());
                }
            }
        } else {
            self.adapter.push(frame.input_buffer().copy_deep());
        }


        //self.process_frames(decoder)
        Ok(())
    }
}
