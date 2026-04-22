mod azure_devops;
mod bitbucket;
mod cli;
mod clone_options;
mod clone_task_manager;
mod filter_options;
mod git;
mod gitea;
mod github;
mod gitlab;
mod process_manager;
mod symlink_manager;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use clone_options::CloneOptions;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let command = match (args.command, args.folder) {
        // Explicit subcommand provided
        (Some(cmd), _) => cmd,
        // No subcommand but folder provided - default to SuperPull
        (None, Some(folder)) => cli::Commands::SuperPull {
            folder,
            recurse: false,
        },
        // Neither provided - show help
        (None, None) => {
            eprintln!("Usage: superpull [OPTIONS] [FOLDER] [COMMAND]");
            eprintln!("       superpull [OPTIONS] <COMMAND>");
            eprintln!("\nRun 'superpull --help' for more information.");
            std::process::exit(1);
        }
    };

    match command {
        cli::Commands::SuperPull { folder, recurse } => {
            git::super_pull(&folder, recurse, args.throttle, args.timeout).await?;
        }
        cli::Commands::AzClone {
            organization,
            folder,
            token,
            server_url,
            name_patterns,
            exclude_patterns,
            max_size_kb,
            create_symlinks,
        } => {
            let options: CloneOptions = CloneOptions::new()
                .with_throttle(args.throttle)
                .with_timeout(args.timeout)
                .with_name_patterns(name_patterns)
                .with_exclude_patterns(exclude_patterns)
                .with_max_size_kb(max_size_kb)
                .with_create_symlinks(create_symlinks);

            azure_devops::super_clone(
                &organization,
                &folder,
                token.as_deref(),
                server_url.as_deref(),
                options,
            )
            .await?;
        }
        cli::Commands::BbClone {
            folder,
            token,
            server_url,
            name_patterns,
            exclude_patterns,
            max_size_kb,
            create_symlinks,
            use_api_v1,
        } => {
            let options = CloneOptions::new()
                .with_throttle(args.throttle)
                .with_timeout(args.timeout)
                .with_name_patterns(name_patterns)
                .with_exclude_patterns(exclude_patterns)
                .with_max_size_kb(max_size_kb)
                .with_create_symlinks(create_symlinks);

            bitbucket::super_clone(
                &folder,
                args.bearer_token,
                token.as_deref(),
                server_url.as_deref(),
                use_api_v1,
                options,
            )
            .await?;
        }
        cli::Commands::GeaClone {
            base_url,
            organization,
            folder,
            token,
            name_patterns,
            exclude_patterns,
            max_size_kb,
            create_symlinks,
        } => {
            let options = CloneOptions::new()
                .with_throttle(args.throttle)
                .with_timeout(args.timeout)
                .with_name_patterns(name_patterns)
                .with_exclude_patterns(exclude_patterns)
                .with_max_size_kb(max_size_kb)
                .with_create_symlinks(create_symlinks);

            gitea::super_clone(&base_url, &organization, &folder, token.as_deref(), options)
                .await?;
        }
        cli::Commands::GhClone {
            entity,
            folder,
            server_url,
            teams,
            name_patterns,
            exclude_patterns,
            max_size_kb,
            create_symlinks,
        } => {
            let options = CloneOptions::new()
                .with_throttle(args.throttle)
                .with_timeout(args.timeout)
                .with_name_patterns(name_patterns)
                .with_exclude_patterns(exclude_patterns)
                .with_max_size_kb(max_size_kb)
                .with_create_symlinks(create_symlinks);

            github::super_clone(
                &entity,
                &folder,
                args.bearer_token,
                server_url.as_deref(),
                teams,
                options,
            )
            .await?;
        }
        cli::Commands::GlClone {
            entity,
            folder,
            token,
            server_url,
            is_group,
            name_patterns,
            exclude_patterns,
            max_size_kb,
            create_symlinks,
        } => {
            let options = CloneOptions::new()
                .with_throttle(args.throttle)
                .with_timeout(args.timeout)
                .with_name_patterns(name_patterns)
                .with_exclude_patterns(exclude_patterns)
                .with_max_size_kb(max_size_kb)
                .with_create_symlinks(create_symlinks);

            gitlab::super_clone(
                &entity,
                &folder,
                is_group,
                token.as_deref(),
                server_url.as_deref(),
                options,
            )
            .await?;
        }
    }

    Ok(())
}
