//
// logger
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[cfg(windows)]
extern crate winapi;

extern crate log;
// ----------------------------------------------------------------------------
pub use log::LevelFilter;
// ----------------------------------------------------------------------------
#[macro_export]
macro_rules! fatal {
    ($msg: expr) => ({
        error!("{}", $msg);
        panic!("previous error");
    });
    ($msg: expr, $($arg:tt)+) => ({
        error!($msg, $($arg)+);
        panic!("previous error");
    });
}
// ----------------------------------------------------------------------------
pub enum ConsoleColor {
    Default,
    Black,
    Blue,
    Green,
    Red,
    Yellow,
    Magenta,
    Cyan,
    White,
}
pub trait ColorConsole {
    fn set_color(col: ConsoleColor);
}
// ----------------------------------------------------------------------------
pub fn pre_init_fatal<S: Into<String>>(msg: S) {
    Console::set_color(ConsoleColor::Red);
    println!("ERROR - {}", msg.into());
    Console::set_color(ConsoleColor::Default);
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use log::{Record, Level, Metadata, SetLoggerError};

#[cfg(not(windows))]
mod ansi_console;
#[cfg(windows)]
mod win_console;

#[cfg(not(windows))]
pub use self::ansi_console::AnsiConsole as Console;
#[cfg(windows)]
pub use self::win_console::WinConsole as Console;

// ----------------------------------------------------------------------------
static LOGGER: SimpleLogger = SimpleLogger;
struct SimpleLogger;
// ----------------------------------------------------------------------------
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            match record.level() {
                Level::Error => Console::set_color(ConsoleColor::Red),
                Level::Warn => Console::set_color(ConsoleColor::Yellow),
                _ => Console::set_color(ConsoleColor::Default)
            };
            println!("{} - {}", record.level(), record.args());
            Console::set_color(ConsoleColor::Default);
        }
    }

    fn flush(&self) {

    }
}
// ----------------------------------------------------------------------------
#[allow(clippy::bind_instead_of_map)]
pub fn init(loglevel: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER)
        .and_then(|_| { log::set_max_level(loglevel); Ok(()) })
}
// ----------------------------------------------------------------------------
