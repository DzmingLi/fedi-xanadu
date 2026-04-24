//! CLI command modules. Each file here owns one `*Command` subcommand enum
//! plus the `handle_*` dispatcher that drives it. `main.rs` stays a thin
//! entry point that parses `Cli` and routes to the right `handle_*`.

pub mod admin;
pub mod book;
pub mod book_series;
pub mod course;
pub mod course_group;
pub mod tree;
pub mod util;
