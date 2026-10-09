mod args;
mod cli;
mod commands;
mod help;
mod output;

#[doc(hidden)]
pub mod test {
    pub use crate::args::__test as cli;
    pub use crate::commands::broker::__test as broker;
}

pub use cli::version;
pub use help::help;

pub fn run(args: Vec<String>) -> Result<(), String> {
    cli::run(args)
}
