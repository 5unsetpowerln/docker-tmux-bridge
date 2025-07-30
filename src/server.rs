use std::net::SocketAddr;
use std::sync::OnceLock;

use anyhow::{Context, Result, anyhow, bail};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use clap::Parser;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

// This can be uninitialized.
static ENTER_COMMAND_RAW: OnceLock<String> = OnceLock::new();

pub const DEFAULT_IP: [u8; 4] = [127, 0, 0, 1];
pub const DEFAULT_PORT: u16 = 3000;

#[derive(Parser, Debug)]
pub struct Args {
    enter_command: Option<String>,
    port: Option<u16>,
}

pub async fn run(args: Args) {
    if let Some(enter_command) = args.enter_command {
        ENTER_COMMAND_RAW.set(enter_command).unwrap();
    }

    let mut port = DEFAULT_PORT;

    if let Some(specified_port) = args.port {
        port = specified_port;
    }

    let app = Router::new().route("/", post(execute_command));

    let addr = SocketAddr::from((DEFAULT_IP, port));
    println!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    tmux_action: TmuxAction,
    command: Option<String>,
    container_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ValueEnum, Clone)]
pub enum TmuxAction {
    SplitWindowVertical,
    SplitWindowHorizontal,
}

impl Request {
    pub fn new(
        tmux_action: TmuxAction,
        command: Option<String>,
        container_id: Option<String>,
    ) -> Self {
        Self {
            tmux_action,
            command,
            container_id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    success: bool,
    message: String,
}

impl Response {
    fn new(success: bool, message: &str) -> Self {
        Self {
            success,
            message: message.to_string(),
        }
    }

    fn json(self) -> Json<Self> {
        Json(self)
    }
}

async fn execute_command(Json(request): Json<Request>) -> impl IntoResponse {
    let enter_command = match construct_enter_command(request.container_id).await {
        Ok(cmd) => cmd,
        Err(err) => {
            return (
                StatusCode::OK,
                Response::new(false, &format!("{err}")).json(),
            );
        }
    };

    let inner_command = if let Some(inner_cmd) = request.command {
        shlex::split(&inner_cmd).unwrap_or_default()
    } else {
        Vec::default()
    };

    let mut command = Command::new("tmux");

    command.arg("split-window");

    if let TmuxAction::SplitWindowHorizontal = request.tmux_action {
        command.arg("-h");
    }

    command.args(enter_command).args(inner_command);

    match command.output().await {
        Ok(output) => {
            let stdout = unsafe { String::from_utf8_unchecked(output.stdout) };
            let stderr = unsafe { String::from_utf8_unchecked(output.stderr) };

            if output.status.success() {
                (
                    StatusCode::OK,
                    Response::new(
                        true,
                        &format!(
                            "Command executed successfully. Stdout: {stdout}, Stderr: {stderr}",
                        ),
                    )
                    .json(),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Response::new(
                        false,
                        &format!(
                            "Command failed with status code: {}. Stdout: {stdout}, Stderr: {stderr}",
                            output.status
                        ),
                    )
                    .json(),
                )
            }
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Response::new(false, &format!("Failed to execute a command: {err}")).json(),
        ),
    }
}

async fn construct_enter_command(container_id: Option<String>) -> Result<Vec<String>> {
    if ENTER_COMMAND_RAW.get().is_none() && container_id.is_none() {
        bail!(anyhow!(
            "No method to enter into the docker container is available."
        ));
    }

    if let Some(enter_command_raw) = ENTER_COMMAND_RAW.get() {
        let enter_command =
            shlex::split(enter_command_raw).context("Failed to split enter_command_raw")?;

        return Ok(enter_command);
    }

    let container_id = container_id.unwrap(); // Presence of container id is already confirmed.

    Ok(vec![
        "docker".to_string(),
        "exec".to_string(),
        "-it".to_string(),
        container_id,
        "/bin/bash".to_string(),
        "-C".to_string(),
    ])
}
