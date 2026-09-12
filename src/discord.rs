use crate::controller::Controller;

use anyhow::Result;
use google_cloud_compute_v1::model::instance::Status;
use poise::{Framework, FrameworkOptions, serenity_prelude::*};

pub struct PoiseState {
    pub controller: Controller,
}

type Context<'a> = poise::Context<'a, PoiseState, anyhow::Error>;

/// Wrapper around [`Controller::get_status`] for Discord.
#[poise::command(slash_command)]
async fn status(ctx: Context<'_>) -> Result<()> {
    use Status::*;

    let Some(status) = ctx.data().controller.get_status().await? else {
        ctx.say("No status returned… :(").await?;
        return Ok(());
    };

    let message = match status {
        Running => "Online",
        Terminated | Stopped => "Offline",
        Provisioning | Staging | Pending => "Starting",
        Deprovisioning | Stopping | PendingStop => "Stopping",
        Suspending => "Suspending",
        Suspended => "Suspended",
        Repairing => "Repairing",

        UnknownValue(v) => {
            return Ok(ctx.say(format!("Unknown value: {v:?}")).await.map(|_| ())?);
        }

        // because it's marked as non-exhaustive
        s => {
            return Ok(ctx
                .say(format!("Uncovered status: {s:?}"))
                .await
                .map(|_| ())?);
        }
    };

    ctx.say(message).await?;

    Ok(())
}

// TODO: Check server status when doing /start and /stop and give appropriate responses.

/// Wrapper around [`Controller::start`] for Discord.
#[poise::command(slash_command)]
async fn start(ctx: Context<'_>) -> Result<()> {
    ctx.defer().await?;
    ctx.data().controller.start_vm().await?;
    ctx.say("Server is up!").await?;

    Ok(())
}

/// Wrapper around [`Controller::stop`] for Discord.
#[poise::command(slash_command)]
async fn stop(ctx: Context<'_>) -> Result<()> {
    ctx.defer().await?;
    ctx.data().controller.stop_vm().await?;
    ctx.say("Server is now shutting down…").await?;

    Ok(())
}

/// Sets up the Discord client.
pub async fn setup(controller: Controller, token: String) -> Result<Client> {
    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![status(), start(), stop()],
            ..Default::default()
        })
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

    let client = ClientBuilder::new(token, GatewayIntents::non_privileged())
        .framework(framework)
        .await?;

    Ok(client)
}
