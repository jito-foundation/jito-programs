use std::{str::FromStr, sync::Arc, time::Duration};

use anchor_lang::AccountDeserialize;
use axum::{
    body::Body,
    error_handling::HandleErrorLayer,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use http::Request;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use tower::{
    buffer::BufferLayer, limit::RateLimitLayer, load_shed::LoadShedLayer, timeout::TimeoutLayer,
    ServiceBuilder,
};
use tower_http::{
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{info, instrument, warn, Span};

use crate::{error, Result};

pub struct RouterState {
    pub jito_tip_distribution_program_id: Pubkey,
    pub rpc_client: RpcClient,
}

impl std::fmt::Debug for RouterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterState")
            .field(
                "jito_tip_distribution_program_id",
                &self.jito_tip_distribution_program_id,
            )
            .field("rpc_client", &self.rpc_client.url())
            .finish()
    }
}

#[instrument]
pub fn get_routes(state: Arc<RouterState>) -> Router {
    let middleware = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(error::handle_error))
        .layer(BufferLayer::new(1000))
        .layer(RateLimitLayer::new(10000, Duration::from_secs(1)))
        .layer(TimeoutLayer::new(Duration::from_secs(20)))
        .layer(LoadShedLayer::new())
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &Request<Body>, _span: &Span| {
                    info!("started {} {}", request.method(), request.uri().path())
                })
                .on_response(
                    DefaultOnResponse::new()
                        .level(tracing_core::Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        );

    let router = Router::new().route("/", get(root)).route(
        "/get_tip_distribution/:vote_account/:epoch",
        get(get_tip_distribution),
    );

    router.layer(middleware).with_state(state)
}

async fn root() -> impl IntoResponse {
    "Jito Programs API"
}

#[derive(Debug, Serialize)]
pub struct TipDistribution {
    /// The validator's vote account, also the recipient of remaining lamports after
    /// upon closing this account.
    pub validator_vote_account: Pubkey,

    /// The only account authorized to upload a merkle-root for this account.
    pub merkle_root_upload_authority: Pubkey,

    /// The merkle root used to verify user claims from this account.
    pub merkle_root: Option<MerkleRoot>,

    /// Epoch for which this account was created.  
    pub epoch_created_at: u64,

    /// The commission basis points this validator charges.
    pub validator_commission_bps: u16,

    /// The epoch (upto and including) that tip funds can be claimed.
    pub expires_at: u64,

    /// The bump used to generate this account
    pub bump: u8,
}

#[derive(Debug, Serialize)]
pub struct MerkleRoot {
    /// The 256-bit merkle root.
    pub root: [u8; 32],

    /// Maximum number of funds that can ever be claimed from this [MerkleRoot].
    pub max_total_claim: u64,

    /// Maximum number of nodes that can ever be claimed from this [MerkleRoot].
    pub max_num_nodes: u64,

    /// Total funds that have been claimed.
    pub total_funds_claimed: u64,

    /// Number of nodes that have been claimed.
    pub num_nodes_claimed: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Params {
    vote_account: String,
    epoch: u64,
}

async fn get_tip_distribution(
    Path(params): Path<Params>,
    State(state): State<Arc<RouterState>>,
) -> Result<impl IntoResponse> {
    let vote_account = Pubkey::from_str(&params.vote_account).unwrap();
    let (tip_distribution_account, _) = Pubkey::find_program_address(
        &[
            jito_tip_distribution::state::TipDistributionAccount::SEED,
            &vote_account.to_bytes(),
            &params.epoch.to_le_bytes(),
        ],
        &state.jito_tip_distribution_program_id,
    );
    let account_data = state
        .rpc_client
        .get_account_data(&tip_distribution_account)
        .await?;
    let tip_distribution = jito_tip_distribution::state::TipDistributionAccount::try_deserialize(
        &mut account_data.as_slice(),
    )
    .unwrap();

    let merkle_root = match tip_distribution.merkle_root {
        Some(inner) => Some(MerkleRoot {
            root: inner.root,
            max_total_claim: inner.max_total_claim,
            max_num_nodes: inner.max_num_nodes,
            total_funds_claimed: inner.total_funds_claimed,
            num_nodes_claimed: inner.num_nodes_claimed,
        }),
        None => None,
    };

    Ok(Json(TipDistribution {
        validator_vote_account: tip_distribution.validator_vote_account,
        merkle_root_upload_authority: tip_distribution.merkle_root_upload_authority,
        merkle_root,
        epoch_created_at: tip_distribution.epoch_created_at,
        validator_commission_bps: tip_distribution.validator_commission_bps,
        expires_at: tip_distribution.expires_at,
        bump: tip_distribution.bump,
    }))
}
