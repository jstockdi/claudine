mod cli;
mod config;
mod docker;
mod init;
mod plugin;
mod project;
mod repo;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::{Cli, Command, PluginCommand};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build => docker::cmd_build(),
        Command::Init { project, ssh_key, repos } => init::cmd_init(&project, ssh_key.as_deref(), &repos),
        Command::Run { project, repo, args } => docker::cmd_run(&project, repo.as_deref(), &args),
        Command::Shell { project, repo } => docker::cmd_shell(&project, repo.as_deref()),
        Command::Destroy { project } => docker::cmd_destroy(&project),
        Command::List => docker::cmd_list(),
        Command::Plugin { command } => match command {
            PluginCommand::Add { project, plugin } => plugin::cmd_plugin_add(&project, &plugin),
            PluginCommand::Remove { project, plugin } => plugin::cmd_plugin_remove(&project, &plugin),
            PluginCommand::List { project } => plugin::cmd_plugin_list(&project),
            PluginCommand::Available => plugin::cmd_plugin_available(),
        },
        Command::Repo { command } => repo::cmd_repo(command),
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "claudine", &mut std::io::stdout());
            Ok(())
        }
    }
}
