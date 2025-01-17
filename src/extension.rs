//! Allow overriding the Spine C Runtime extension functions.
//!
//! Because this library is primarily a wrapper around the official C runtime, it tries to mimic
//! the usage of that runtime as closely as possible. One awkward outcome of this goal is that the
//! C runtime expects you to implement a few functions directly. This module allows you to set
//! those callbacks which have been wrapped in Rust.
//!
//! The callbacks from C are:
//!
//! * `void _spAtlasPage_createTexture (spAtlasPage* self, const char* path);`
//! * `void _spAtlasPage_disposeTexture (spAtlasPage* self);`
//! * `char* _spUtil_readFile (const char* path, int* length);`
//!
//! They can be set with the functions found on this page.
//!
//! You can read more about these functions on the
//! [spine-c Runtime Docs](http://en.esotericsoftware.com/spine-c#Integrating-spine-c-in-your-engine).

use std::ffi::CStr;
use std::fs::read;
use std::sync::{Arc, Mutex, Once};

use crate::c::{c_int, c_void, size_t};
use crate::c_interface::NewFromPtr;
use crate::{
    atlas::AtlasPage,
    c::{c_char, spAtlasPage},
};

#[derive(Default)]
pub(crate) struct Extension {
    create_texture_cb: Option<Box<dyn Fn(&mut AtlasPage, &str)>>,
    dispose_texture_cb: Option<Box<dyn Fn(&mut AtlasPage)>>,
    read_file_cb: Option<Box<dyn Fn(&str) -> Option<Vec<u8>>>>,
}

impl Extension {
    fn singleton() -> Arc<Mutex<Extension>> {
        static START: Once = Once::new();
        static mut INSTANCE: Option<Arc<Mutex<Extension>>> = None;
        START.call_once(|| unsafe {
            INSTANCE = Some(Arc::new(Mutex::new(Extension::default())));
        });
        unsafe {
            let singleton = INSTANCE.as_ref().unwrap();
            singleton.clone()
        }
    }
}

/// Set `_spAtlasPage_createTexture`
///
/// The purpose of this callback is to allow loading textures in whichever engine is being used.
/// The following example shows the intended usage by storing the texture on the renderer object
/// of the AtlasPage which can be acquired later.
/// ```
/// struct SpineTexture(pub String);
///
/// fn main() {
///     rusty_spine::extension::set_create_texture_cb(|atlas_page, path| {
///         atlas_page.renderer_object().set(SpineTexture(path.to_owned()));
///     });
///     rusty_spine::extension::set_dispose_texture_cb(|atlas_page| unsafe {
///         atlas_page.renderer_object().dispose::<SpineTexture>();
///     });
/// }
/// ```
pub fn set_create_texture_cb<F>(create_texture_cb: F)
where
    F: Fn(&mut AtlasPage, &str) + 'static,
{
    let singleton = Extension::singleton();
    let mut extension = singleton.lock().unwrap();
    extension.create_texture_cb = Some(Box::new(create_texture_cb));
}

/// Set `_spAtlasPage_disposeTexture`
///
/// For an example, see [set_create_texture_cb](fn.set_create_texture_cb.html).
pub fn set_dispose_texture_cb<F>(dispose_texture_cb: F)
where
    F: Fn(&mut AtlasPage) + 'static,
{
    let singleton = Extension::singleton();
    let mut extension = singleton.lock().unwrap();
    extension.dispose_texture_cb = Some(Box::new(dispose_texture_cb));
}

/// Set `_spUtil_readFile`
///
/// Can be used to customize file loading when using functions which read files from disk. This
/// callback is largely unnecessary as its possible to avoid calling these sorts of functions
/// if read-from-disk is not desirable. Additionally, a default implementation using Rust's
/// `std::fs::read` is provided if this callback remains unset.
///
/// ```
/// fn main() {
///     rusty_spine::extension::set_read_file_cb(|path| {
///         std::fs::read(path).ok()
///     });
/// }
/// ```
pub fn set_read_file_cb<F>(read_file_cb: F)
where
    F: Fn(&str) -> Option<Vec<u8>> + 'static,
{
    let singleton = Extension::singleton();
    let mut extension = singleton.lock().unwrap();
    extension.read_file_cb = Some(Box::new(read_file_cb));
}

#[no_mangle]
extern "C" fn _spAtlasPage_createTexture(c_atlas_page: *mut spAtlasPage, c_path: *const c_char) {
    let singleton = Extension::singleton();
    let extension = singleton.lock().unwrap();
    if let Some(cb) = &extension.create_texture_cb {
        unsafe {
            cb(
                &mut AtlasPage::new_from_ptr(c_atlas_page),
                CStr::from_ptr(c_path).to_str().unwrap(),
            );
        }
    }
}

#[no_mangle]
extern "C" fn _spAtlasPage_disposeTexture(c_atlas_page: *mut spAtlasPage) {
    let singleton = Extension::singleton();
    let extension = singleton.lock().unwrap();
    if let Some(cb) = &extension.dispose_texture_cb {
        unsafe {
            cb(&mut AtlasPage::new_from_ptr(c_atlas_page));
        }
    }
}

extern "C" {
    fn spine_malloc(__size: size_t) -> *mut c_void;
    fn spine_memcpy(__dest: *mut c_void, __src: *const c_void, __n: size_t) -> *mut c_void;
}

#[no_mangle]
extern "C" fn _spUtil_readFile(c_path: *const c_char, c_length: *mut c_int) -> *mut c_char {
    let singleton = Extension::singleton();
    let extension = singleton.lock().unwrap();
    if let Some(cb) = &extension.read_file_cb {
        if let Some(data) = cb(unsafe { CStr::from_ptr(c_path).to_str().unwrap() }) {
            unsafe {
                *c_length = data.len() as c_int;
                let c_data = spine_malloc(data.len() as size_t);
                spine_memcpy(c_data, data.as_ptr() as *const c_void, data.len() as size_t);
                c_data as *mut c_char
            }
        } else {
            std::ptr::null_mut()
        }
    } else {
        let str = unsafe { CStr::from_ptr(c_path).to_str().unwrap().to_owned() };
        if let Ok(data) = read(str) {
            let c_data = unsafe { spine_malloc(data.len() as size_t) };
            unsafe {
                spine_memcpy(c_data, data.as_ptr() as *const c_void, data.len() as size_t);
                *c_length = data.len() as c_int;
            }
            c_data as *mut c_char
        } else {
            std::ptr::null_mut()
        }
    }
}
