use std::net::Ipv4Addr;

use crate::server;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use regex::Regex;
use tokio::fs;
use tokio::io;
use tokio::io::AsyncBufReadExt;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(short, long)]
    ip: Option<Ipv4Addr>,
    #[arg(short, long)]
    port: Option<u16>,
    #[arg(short, long)]
    tmux_action: server::TmuxAction,
    command: Option<String>,
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

    let container_id = get_container_id()
        .await
        .expect("Failed to get the container id.");
    let url = format!("http://{ip}:{port}/execute");
    let request = server::Request::new(args.tmux_action, args.command, container_id);

    println!("Request: {request:?}");
    println!("Requesting to {url}...");

    let client = reqwest::Client::new();

    let result = client.post(url).json(&request).send().await;

    match result {
        Ok(response) => {
            println!("{}", response.text().await.unwrap());
        }
        Err(err) => {
            println!("Failed to send a request: {err}");
        }
    }
}

async fn get_container_id() -> Result<Option<String>> {
    let file = fs::File::open("/proc/self/mountinfo")
        .await
        .context("Failed to open the /proc/self/mountinfo.")?;
    let reader = io::BufReader::new(file);

    let re = Regex::new("[0-9a-f]{64}").context("Failed to create a regex.")?;

    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.context("Failed to read line.")? {
        if line.contains("/docker/containers/") {
            if let Some(mat) = re.find(&line) {
                return Ok(Some(mat.as_str().to_string()));
            }
        }
    }

    Ok(None)
}
