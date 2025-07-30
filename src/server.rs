use std::net::SocketAddr;
use std::sync::{Mutex, OnceLock};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

static ENTER_COMMAND_RAW: OnceLock<String> = OnceLock::new();

pub const DEFAULT_IP: [u8; 4] = [127, 0, 0, 1];
pub const DEFAULT_PORT: u16 = 3000;
pub const EXECUTE_ENDPOINT_NAME: &str = "execute";

#[derive(Parser, Debug)]
pub struct Args {
    enter_command: String,
    port: Option<u16>,
}

pub async fn run(args: Args) {
    ENTER_COMMAND_RAW.set(args.enter_command).unwrap();

    let mut port = DEFAULT_PORT;

    if let Some(specified_port) = args.port {
        port = specified_port;
    }

    let app = Router::new()
        .route("/", get(root))
        .route(&format!("/{EXECUTE_ENDPOINT_NAME}"), post(execute_command));

    let addr = SocketAddr::from((DEFAULT_IP, port));
    println!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, Axum!"
}

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
pub struct CommandRequest {
    #[arg(short, long)]
    tmux_action: TmuxAction,
    #[arg(short, long)]
    command: String,
}

#[derive(Debug, Serialize, Deserialize, ValueEnum, Clone)]
pub enum TmuxAction {
    SplitWindowVertical,
    SplitWindowHorizontal,
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

async fn execute_command(Json(request): Json<CommandRequest>) -> impl IntoResponse {
    let enter_command =
        shlex::split(ENTER_COMMAND_RAW.wait()).expect("Enter command can't be None.");

    let inner_command = shlex::split(&request.command).unwrap_or_default();

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
