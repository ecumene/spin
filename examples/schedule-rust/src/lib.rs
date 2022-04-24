use anyhow::{anyhow, Result};
use spin_sdk::{
    http::{internal_server_error, Request, Response},
    http_component, redis,
};

#[http_component]
fn publish(_req: Request) -> Result<Response> {
    return Ok(http::Response::builder().status(200).body(None)?);
}
