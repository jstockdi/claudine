use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claudine", version, about = "Run Claude Code in isolated Docker containers")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new claudine project configuration
    Init {
        /// Name of the project to initialize
        project: String,
    },

    /// Run Claude Code in a container for the given project
    Run {
        /// Name of the project to run
        project: String,

        /// Additional arguments passed through to Claude
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Open an interactive shell in a project's container
    Shell {
        /// Name of the project
        project: String,
    },

    /// Destroy a project's container and associated resources
    Destroy {
        /// Name of the project to destroy
        project: String,
    },

    /// Build the claudine Docker image
    Build,

    /// List all claudine projects
    List,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
}
