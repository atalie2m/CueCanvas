use anyhow::Context;
use cuecanvas_runtime::{addr_for_port, app, default_state};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = addr_for_port(configured_port());
    let state = default_state();
    let tokens = { state.lock().await.tokens.clone() };
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind CueCanvas runtime at {addr}"))?;
    let actual_addr = listener
        .local_addr()
        .context("failed to read CueCanvas runtime listener address")?;
    println!("CueCanvas runtime listening on http://{actual_addr}");
    println!("Editor token: {}", tokens.editor_token);
    println!(
        "Program overlay URL: http://{actual_addr}/overlay/program?token={}",
        tokens.overlay_token
    );
    axum::serve(listener, app(state))
        .await
        .context("CueCanvas runtime failed")?;
    Ok(())
}

fn configured_port() -> u16 {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--port" {
            return args
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(4317);
        }
        if let Some(value) = arg.strip_prefix("--port=") {
            return value.parse().unwrap_or(4317);
        }
    }
    std::env::var("CUECANVAS_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4317)
}
