mod client;
mod server;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    sub_command: SubCommand,
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    Server(server::Args),
    Client(client::Args),
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.sub_command {
        SubCommand::Client(args) => {
            client::run(args).await;
        }
        SubCommand::Server(args) => {
            server::run(args).await;
        }
    }
}
