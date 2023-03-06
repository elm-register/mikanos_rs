use core::fmt::Write;

use crate::gop::console::console_builder::ConsoleBuilder;
use crate::gop::console::console_writer::ConsoleWriter;
use common_lib::frame_buffer::FrameBufferConfig;
use spin::Mutex;

pub mod console_builder;
pub mod console_writer;

pub struct GlobalConsole(Option<Mutex<ConsoleWriter>>);
pub static mut CONSOLE: GlobalConsole = GlobalConsole(None);

impl GlobalConsole {
    pub fn init(&mut self, frame_buffer_config: FrameBufferConfig) {
        let console = ConsoleBuilder::new().build(frame_buffer_config);
        self.0 = Some(Mutex::new(console));
    }

    pub fn get_mut(&mut self) -> &mut ConsoleWriter {
        self.0.as_mut().unwrap().get_mut()
    }
}

pub fn init_console(frame_buffer_config: FrameBufferConfig) {
    unsafe { CONSOLE.init(frame_buffer_config) };
}

pub fn get_mut_console() -> &'static mut ConsoleWriter {
    unsafe { CONSOLE.get_mut() }
}

#[doc(hidden)]
pub fn _print(s: core::fmt::Arguments) {
    get_mut_console().write_fmt(s).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => ($crate::gop::console::_print(format_args!($($args)*)));
}

#[macro_export]
macro_rules! println {
        () => {
            $crate::print!("\n");
        };
        ($fmt: expr) => {
           $crate::print!(concat!($fmt, "\n"));
       };
       ($fmt: expr, $($args:tt)*) => {
           $crate::print!("{}\n", format_args!($($args)*));
       };
}
