// Copyright (C) 2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ptr;
use std::mem;

use glib_ffi;
use gobject_ffi;
use gst_ffi;
use gst_video_ffi;

use glib;
use glib::translate::*;
use gst;
use gst::prelude::*;

use gst_video;
use gst_video::prelude::*;

use object::*;
use element::*;
use anyimpl::*;

pub trait VideoDecoderImpl<T: VideoDecoderBase>
    : AnyImpl + ObjectImpl<T> + ElementImpl<T> + Send + Sync + 'static {
    fn start(&self, _element: &T) -> bool {
        true
    }

    fn finish(&self, _element: &T) -> gst::FlowReturn {
        gst::FlowReturn::Ok
    }


    fn stop(&self, _element: &T) -> bool {
        true
    }

    fn set_format(&self, _element: &T, state: &gst_video::VideoCodecState) -> bool {
        unimplemented!()
    }

    fn handle_frame(&self, _element: &T, frame: &gst_video::VideoCodecFrame) -> gst::FlowReturn {
        unimplemented!()
    }
}

any_impl!(VideoDecoderBase, VideoDecoderImpl);

pub unsafe trait VideoDecoderBase
    : IsA<gst::Element> + IsA<gst_video::VideoDecoder> + ObjectType {
}

// Class overrides
pub unsafe trait VideoDecoderClassExt<T: VideoDecoderBase>
where
    T::ImplType: VideoDecoderImpl<T>,
{
    fn override_vfuncs(&mut self, _: &ClassInitToken) {
        unsafe {
            let klass = &mut *(self as *const Self as *mut gst_video_ffi::GstVideoDecoderClass);
            klass.start = Some(video_decoder_start::<T>);
            klass.stop = Some(video_decoder_stop::<T>);
            klass.set_format = Some(video_decoder_set_format::<T>);
            klass.handle_frame = Some(video_decoder_handle_frame::<T>);
            klass.finish = Some(video_decoder_finish::<T>);
        }
    }
}

glib_wrapper! {
    pub struct VideoDecoder(Object<InstanceStruct<VideoDecoder>>): [
        gst_video::VideoDecoder => gst_video_ffi::GstVideoDecoder,
        gst::Element => gst_ffi::GstElement,
        gst::Object => gst_ffi::GstObject,
    ];

    match fn {
        get_type => || get_type::<VideoDecoder>(),
    }
}

unsafe impl<T: IsA<gst::Element> + IsA<gst_video::VideoDecoder> + ObjectType> VideoDecoderBase for T {}
pub type VideoDecoderClass = ClassStruct<VideoDecoder>;

// FIXME: Boilerplate
unsafe impl VideoDecoderClassExt<VideoDecoder> for VideoDecoderClass {}
unsafe impl ElementClassExt<VideoDecoder> for VideoDecoderClass {}

#[macro_export]
macro_rules! box_video_decoder_impl(
    ($name:ident) => {
        box_element_impl!($name);

        impl<T: VideoDecoderBase> VideoDecoderImpl<T> for Box<$name<T>> {
            fn start(&self, element: &T) -> bool {
                let imp: &$name<T> = self.as_ref();
                imp.start(element)
            }

            fn finish(&self, element: &T) -> gst::FlowReturn {
                let imp: &$name<T> = self.as_ref();
                imp.finish(element)
            }

            fn stop(&self, element: &T) -> bool {
                let imp: &$name<T> = self.as_ref();
                imp.stop(element)
            }

            fn set_format(&self, element: &T, state: &gst_video::VideoCodecState) ->
                    bool {
                let imp: &$name<T> = self.as_ref();
                imp.set_format(element, state)
            }

            fn handle_frame(&self, element: &T, frame: &gst_video::VideoCodecFrame) ->
                gst::FlowReturn {
                let imp: &$name<T> = self.as_ref();
                imp.handle_frame(element, frame)
            }
        }
    };
);
box_video_decoder_impl!(VideoDecoderImpl);

impl ObjectType for VideoDecoder {
    const NAME: &'static str = "VideoDecoder";
    type GlibType = gst_video_ffi::GstVideoDecoder;
    type GlibClassType = gst_video_ffi::GstVideoDecoderClass;
    type ImplType = Box<VideoDecoderImpl<Self>>;

    fn glib_type() -> glib::Type {
        unsafe { from_glib(gst_video_ffi::gst_video_decoder_get_type()) }
    }

    fn class_init(token: &ClassInitToken, klass: &mut VideoDecoderClass) {
        ElementClassExt::override_vfuncs(klass, token);
        VideoDecoderClassExt::override_vfuncs(klass, token);
    }

    object_type_fns!();
}


// Trampolines
unsafe extern "C" fn video_decoder_start<T: VideoDecoderBase>(
    ptr: *mut gst_video_ffi::GstVideoDecoder,
) -> glib_ffi::gboolean
where
    T::ImplType: VideoDecoderImpl<T>,
{
    callback_guard!();
    floating_reference_guard!(ptr);
    let element = &*(ptr as *mut InstanceStruct<T>);
    let wrap: T = from_glib_borrow(ptr as *mut InstanceStruct<T>);
    let imp = &*element.imp;

    panic_to_error!(&wrap, &element.panicked, false, { imp.start(&wrap) }).to_glib()
}

unsafe extern "C" fn video_decoder_finish<T: VideoDecoderBase>(
    ptr: *mut gst_video_ffi::GstVideoDecoder,
) -> gst_ffi::GstFlowReturn
where
    T::ImplType: VideoDecoderImpl<T>,
{
    callback_guard!();
    floating_reference_guard!(ptr);
    let element = &*(ptr as *mut InstanceStruct<T>);
    let wrap: T = from_glib_borrow(ptr as *mut InstanceStruct<T>);
    let imp = &*element.imp;

    panic_to_error!(&wrap, &element.panicked,
                    gst::FlowReturn::Error, { imp.finish(&wrap) }).to_glib()
}

unsafe extern "C" fn video_decoder_stop<T: VideoDecoderBase>(
    ptr: *mut gst_video_ffi::GstVideoDecoder,
) -> glib_ffi::gboolean
where
    T::ImplType: VideoDecoderImpl<T>,
{
    callback_guard!();
    floating_reference_guard!(ptr);
    let element = &*(ptr as *mut InstanceStruct<T>);
    let wrap: T = from_glib_borrow(ptr as *mut InstanceStruct<T>);
    let imp = &*element.imp;

    panic_to_error!(&wrap, &element.panicked, false, { imp.stop(&wrap) }).to_glib()
}

unsafe extern "C" fn video_decoder_set_format<T: VideoDecoderBase>(
    ptr: *mut gst_video_ffi::GstVideoDecoder,
    state: *mut gst_video_ffi::GstVideoCodecState,
) -> glib_ffi::gboolean
where
    T::ImplType: VideoDecoderImpl<T>,
{
    callback_guard!();
    floating_reference_guard!(ptr);
    let element = &*(ptr as *mut InstanceStruct<T>);
    let wrap: T = from_glib_borrow(ptr as *mut InstanceStruct<T>);
    let imp = &*element.imp;
    // FIXME
    let state = from_glib_borrow(state);

    panic_to_error!(&wrap, &element.panicked, false, {
        imp.set_format(&wrap, &state)
    }).to_glib()
}

unsafe extern "C" fn video_decoder_handle_frame<T: VideoDecoderBase>(
    ptr: *mut gst_video_ffi::GstVideoDecoder,
    frame: *mut gst_video_ffi::GstVideoCodecFrame,
) -> gst_ffi::GstFlowReturn
where
    T::ImplType: VideoDecoderImpl<T>,
{
    callback_guard!();
    floating_reference_guard!(ptr);
    let element = &*(ptr as *mut InstanceStruct<T>);
    let wrap: T = from_glib_borrow(ptr as *mut InstanceStruct<T>);
    let imp = &*element.imp;
    // FIXME
    let frame = from_glib_borrow(frame);

    panic_to_error!(&wrap, &element.panicked, gst::FlowReturn::Error, {
        imp.handle_frame(&wrap, &frame)
    }).to_glib()
}
