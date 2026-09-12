#![warn(clippy::pedantic)]
#![allow(clippy::enum_glob_use)]

mod controller;
mod discord;

use crate::controller::{Controller, SecretId};

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let controller = Controller::new().await?;
    let token = controller.get_secret(&SecretId::Discord).await?;
    let mut client = discord::setup(controller, token).await?;

    client.start().await?;

    Ok(())
}
