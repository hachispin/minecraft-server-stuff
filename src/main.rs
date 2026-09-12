#![warn(clippy::pedantic)]
#![allow(clippy::enum_glob_use)]

mod controller;
mod discord;

use crate::{
    controller::{Controller, SecretId},
    discord::{PoiseState, framework_options},
};

use anyhow::Result;
use poise::{Framework, serenity_prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    let controller = Controller::new().await?;
    let token = controller.get_secret(&SecretId::Discord).await?;
    let intents = GatewayIntents::non_privileged();

    let framework = Framework::builder()
        .options(framework_options())
        // ideally would be in discord.rs but it's annoying
        .setup(
            // called when discord connects. registering guild-specific may be faster?
            move |ctx, _ready, framework| {
                Box::pin(async move {
                    let controller = controller;
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(PoiseState { controller })
                })
            },
        )
        .build();

    let mut client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    client.start().await?;

    Ok(())
}
