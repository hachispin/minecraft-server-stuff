use crate::controller::Controller;

use anyhow::Result;
use google_cloud_compute_v1::model::instance::Status;
use poise::FrameworkOptions;

pub struct PoiseState {
    pub controller: Controller,
}

type Context<'a> = poise::Context<'a, PoiseState, anyhow::Error>;

/// Wrapper around [`get_status`] for Discord.
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

pub fn framework_options() -> FrameworkOptions<PoiseState, anyhow::Error> {
    FrameworkOptions {
        commands: vec![status()],
        ..Default::default()
    }
}
