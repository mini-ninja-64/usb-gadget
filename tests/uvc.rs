mod common;
use std::io::stdin;

use common::*;
use usb_gadget::function::uvc::{Frame, Uvc, UvcBuilder};

fn wait() {
    let mut buff = String::new();
    stdin().read_line(&mut buff).expect("Err");
}

#[test]
fn uvc() {
    init();

    let mut builder = Uvc::builder();
    builder.add_frame(&Frame {
        format: "mjpeg",
        name: "mjpeg",
        width: 1920,
        height: 1080,
        frame_intervals: vec![UvcBuilder::fps(15)]
    });
    let (uvc, func) = builder.build();

    let reg = reg(func);
    let p = uvc.get_v4l_device();
    println!("v4l_dev: {}", p.unwrap().display());
    wait();
    unreg(reg).unwrap();
}
