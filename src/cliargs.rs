// use clap::{Parser, Subcommand};
use clap::Parser;

const LONG_ABOUT: &str = r#"
EdgeLink Daemon Program

EdgeLink is a Node-RED compatible run-time engine implemented in Rust.

By replacing the original NodeJS backend with this Rust-based implementation,
you can achieve better performance and a smaller memory footprint.

In summary, you can first test the workflow on a normal desktop PC,
and subsequently deploy EdgeLink along with the `flows.json` workflow file
to an edge computing device that is constrained by limited resources for operational execution.

Copyright (C) 2023-TODAY Li Wei and contributors.
All rights reserved.

For more information, visit the website: https://github.com/oldrev/edgelink
"#;

#[derive(Parser, Debug, Clone)]
#[command(version, about, author, long_about=LONG_ABOUT)]
pub struct CliArgs {
    /// Home directory of EdgeLink, default is `~/.edgelink`
    #[arg(long)]
    pub home: Option<String>,

    /// Path of the 'flows.json' file.
    #[arg(short, long, default_value_t = default_flows_path())]
    pub flows_path: String,

    /// Path of the log configuration file.
    #[arg(short, long)]
    pub log_path: Option<String>,

    /// Verbose level.
    #[arg(short, long, default_value_t = 2)]
    pub verbose: usize,

    /// Read workflow JSON from stdin.
    #[arg(short, long, default_value_t = false)]
    pub stdin: bool,

    /// Set the running environment in 'dev' or 'prod', default is `dev`
    #[arg(long)]
    pub env: Option<String>,
}

fn default_flows_path() -> String {
    dirs_next::home_dir()
        .expect("Can not found the $HOME dir!!!")
        .join(".node-red")
        .join("flows.json")
        .to_string_lossy()
        .to_string()
}