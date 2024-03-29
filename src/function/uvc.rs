//! USB Video Class (UVC) function.
//!
//! The Linux kernel configuration option `CONFIG_USB_CONFIGFS_F_UVC` must be enabled.

use std::{
    ffi::{OsStr, OsString}, fs, io::Result, os::unix::fs::symlink, path::PathBuf
};

use super::{
    util::FunctionDir,
    Function, Handle,
};

pub(crate) fn driver() -> &'static OsStr {
    OsStr::new("uvc")
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub format: &'static str,
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub frame_intervals: Vec<u32>,
}

/// Builder for USB human interface device (HID) function.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct UvcBuilder {
    /// HID subclass to use.
    pub frames: Vec<Frame>
}

impl UvcBuilder {
    pub fn fps(fps: u32) -> u32 {
        let frame_interval = 10000000.0 / fps as f64;
        frame_interval as u32
    }

    pub fn add_frame(&mut self, frame: &Frame) -> &mut UvcBuilder {
        self.frames.push(frame.clone());
        self
    }

    // pub fn build(&mut self) -> (Other, Handle) {
    //         let (other, handle) = self.builder.to_owned().build();
    //         return (other, handle);
    // }

    /// Build the USB function.
    ///
    /// The returned handle must be added to a USB gadget configuration.
    pub fn build(self) -> (Uvc, Handle) {
        let dir = FunctionDir::new();
        let uvc = Uvc { dir: dir.clone() };
        (uvc, Handle::new(UvcFunction { builder: self, dir }))
    }
}

#[derive(Debug)]
struct UvcFunction {
    builder: UvcBuilder,
    dir: FunctionDir,
}

fn add_unix_line_ending(str: &String) -> String {
    let mut str_copy = str.clone();
    str_copy.extend(['\n'].iter());
    return str_copy;
}

impl Function for UvcFunction {
    fn driver(&self) -> OsString {
        driver().into()
    }

    fn dir(&self) -> FunctionDir {
        self.dir.clone()
    }

    fn register(&self) -> Result<()> {
        self.dir.create_dir("streaming/header/h")?;
        let mut sym_links: Vec<(PathBuf, PathBuf)> = Vec::new();

        self.dir.write("streaming_interval", "1\n".as_bytes())?;
        self.dir.write("streaming_maxpacket", "3072\n".as_bytes())?;
        self.dir.write("streaming_maxburst", "1\n".as_bytes())?;

        // Generate frames
        for frame in &self.builder.frames {
            let frame_dir: PathBuf = format!("streaming/{}/{}", frame.format, frame.name).into();
            let frame_path = frame_dir.join(format!("{}p", frame.height));

            self.dir.write(
                frame_path.join("wWidth"),
                add_unix_line_ending(&frame.width.to_string()).as_bytes()
            )?;

            self.dir.write(
                frame_path.join("wHeight"),
                add_unix_line_ending(&frame.height.to_string()).as_bytes()
            )?;

            let frame_buffer_file = (frame.width * frame.height * 2).to_string();
            self.dir.write(
                frame_path.join("dwMaxVideoFrameBufferSize"),
                add_unix_line_ending(&frame_buffer_file).as_bytes()
            )?;

            let interval_file = frame.frame_intervals.iter()
                .map(|interval| interval.to_string())
                .collect::<Vec<String>>()
                .join("\n");
            self.dir.write(
                frame_path.join("dwFrameInterval"),
                add_unix_line_ending(&interval_file).as_bytes()
            )?;

            sym_links.push((frame_dir, format!("streaming/header/h/{}", frame.name).into()));
        }
        
        for usb_speed in ["fs", "hs", "ss"] {
            sym_links.push(("streaming/header/h".into(), format!("streaming/class/{}/h", usb_speed).into()));
        }

        self.dir.create_dir("control/header/h")?;
        for usb_speed in ["fs", "ss"] {
            sym_links.push(("control/header/h".into(), format!("control/class/{}/h", usb_speed).into()))
        }


        // Link headers
        for (original, link) in &sym_links {
            let original = self.dir.property_path(original)?;
            let link = self.dir.property_path(link)?;
            symlink(original, link)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Uvc {
    dir: FunctionDir,
}

impl Uvc {
    pub fn builder() -> UvcBuilder {
        return UvcBuilder { frames: Vec::new() };
    }
}

fn walk_and_delete(dir: PathBuf) -> Result<()> {
    for path in fs::read_dir(dir)? {
        let Ok(path) = path else { continue };
        let path = path.path();
        if path.exists() && path.is_symlink() {
            fs::remove_file(path)?;
        } else if path.is_dir() {
            walk_and_delete(path.clone())?;
            let _ = fs::remove_dir(path);
        }
    }
    Ok(())
}

pub(crate) fn remove_handler(dir: PathBuf) -> Result<()> {
    walk_and_delete(dir.join("control/class"))?;
    walk_and_delete(dir.join("streaming/class"))?;
    walk_and_delete(dir.join("streaming/header"))?;

    walk_and_delete(dir.join("streaming"))?;

    walk_and_delete(dir.join("control/header"))?;
    Ok(())
}