mod error;
mod router;

use std::{net::SocketAddr, str::FromStr, sync::Arc};

use clap::Parser;
use error::ApiError;
use router::RouterState;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use tracing::{info, instrument};

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Bind address for the server
    #[clap(long, env, default_value_t = SocketAddr::from_str("0.0.0.0:7001").unwrap())]
    bind_addr: SocketAddr,

    /// RPC url
    #[clap(long, env)]
    rpc_url: String,

    /// Program ID
    #[clap(long, env)]
    jito_tip_distribution_program_id: Pubkey,
}

#[tokio::main]
#[instrument]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    tracing_subscriber::fmt().init();

    info!("args: {:?}", args);

    info!("starting server at {}", args.bind_addr);

    let rpc_client = RpcClient::new(args.rpc_url.clone());
    info!("started rpc client at {}", args.rpc_url);

    let state = Arc::new(RouterState {
        jito_tip_distribution_program_id: args.jito_tip_distribution_program_id,
        rpc_client,
    });

    let app = router::get_routes(state);

    let listener = tokio::net::TcpListener::bind(&args.bind_addr)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
