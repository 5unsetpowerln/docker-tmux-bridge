use std::net::Ipv4Addr;

use crate::server;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[command(flatten)]
    request: server::CommandRequest,
    #[arg(short, long)]
    ip: Option<Ipv4Addr>,
    #[arg(short, long)]
    port: Option<u16>,
}

pub async fn run(args: Args) {
    let mut ip = Ipv4Addr::from(server::DEFAULT_IP);
    let mut port = server::DEFAULT_PORT;

    if let Some(specified_ip) = args.ip {
        ip = specified_ip
    }

    if let Some(specified_port) = args.port {
        port = specified_port;
    }

    let url = format!("http://{}:{}/{}", ip, port, server::EXECUTE_ENDPOINT_NAME);

    println!("Requesting to {url}...");

    let client = reqwest::Client::new();

    let result = client.post(url).json(&args.request).send().await;

    match result {
        Ok(response) => {
            println!("{}", response.text().await.unwrap());
        }
        Err(err) => {
            println!("Failed to send a request: {err}");
        }
    }
}
