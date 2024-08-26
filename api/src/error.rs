use std::convert::Infallible;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    BoxError, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_program::pubkey::ParsePubkeyError;
use solana_rpc_client_api::client_error::Error as RpcError;
use thiserror::Error;
use tracing::log::error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Rpc Error")]
    RpcError(#[from] RpcError),

    #[error("Parse Pubkey Error")]
    ParsePubkeyError(#[from] ParsePubkeyError),

    #[error("Internal Error")]
    InternalError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    pub error: String,
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::ParsePubkeyError(e) => {
                error!("Parse pubkey error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Pubkey parse error")
            }
            ApiError::RpcError(e) => {
                error!("Rpc error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Rpc error")
            }
            ApiError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
        };
        (
            status,
            Json(Error {
                error: error_message.to_string(),
            }),
        )
            .into_response()
    }
}

pub async fn handle_error(error: BoxError) -> Result<impl IntoResponse, Infallible> {
    if error.is::<tower::timeout::error::Elapsed>() {
        return Ok((
            StatusCode::REQUEST_TIMEOUT,
            Json(json!({
                "code" : 408,
                "error" : "Request Timeout",
            })),
        ));
    };
    if error.is::<tower::load_shed::error::Overloaded>() {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "code" : 503,
                "error" : "Service Unavailable",
            })),
        ));
    }

    Ok((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "code" : 500,
            "error" : "Internal Server Error",
        })),
    ))
}
