//
// quick hack for colors in ansi consoles
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub struct AnsiConsole;
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use super::ConsoleColor as Col;
use super::ColorConsole as Console;
// ----------------------------------------------------------------------------
impl Console for AnsiConsole {
    fn set_color(col: Col) {
        let col = match col {
            Col::Red => "\x1B[31m",
            Col::Green => "\x1B[32m",
            Col::Yellow => "\x1B[33m",
            Col::Blue => "\x1B[34m",
            Col::Magenta => "\x1B[35m",
            Col::Cyan => "\x1B[36m",
            _ => "\x1B[0m",
        };
        print!("{}", col);
    }
}
// ----------------------------------------------------------------------------
