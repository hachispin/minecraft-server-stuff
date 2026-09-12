#![warn(clippy::pedantic)]
#![allow(clippy::enum_glob_use)]

use anyhow::{Result, anyhow};
use google_cloud_compute_v1::{client::Instances, model::instance::Status};
use google_cloud_secretmanager_v1::{client::SecretManagerService, model::SecretPayload};
use poise::{Framework, FrameworkOptions, serenity_prelude::*};

type Context<'a> = poise::Context<'a, (), anyhow::Error>;

const PROJECT: &str = "project-c863d0a5-25e6-435f-8b4";
const NAME: &str = "minecraft-server";
const ZONE: &str = "europe-west1-c";

enum SecretId {
    Discord,
    #[expect(dead_code)]
    DeSec,
}

impl SecretId {
    fn as_str(&self) -> &str {
        use SecretId::*;

        match self {
            Discord => "discord-bot-controller-interface-factory-amazing-wow",
            DeSec => "deSEC-dns-so-i-dont-have-to-type-new-addr-every-time",
        }
    }
}

async fn get_secret(secret_id: &SecretId) -> Result<Option<SecretPayload>> {
    let secret_id = secret_id.as_str();
    let service = SecretManagerService::builder().build().await?;

    let response = service
        .access_secret_version()
        .set_name(format!(
            "projects/{PROJECT}/secrets/{secret_id}/versions/latest"
        ))
        .send()
        .await?;

    Ok(response.payload)
}

async fn get_status() -> Result<Option<Status>> {
    let client = Instances::builder().build().await?;

    let response = client
        .get()
        .set_project(PROJECT)
        .set_zone(ZONE)
        .set_instance(NAME)
        .send()
        .await?;

    Ok(response.status)
}

#[poise::command(slash_command)]
async fn status(ctx: Context<'_>) -> Result<()> {
    use Status::*;

    let Some(status) = get_status().await? else {
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

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    let framework_options = FrameworkOptions {
        commands: vec![status()],
        ..Default::default()
    };

    let framework = Framework::builder()
        .options(framework_options)
        .setup(
            // called when discord connects. registering guild-specific may be faster?
            |ctx, _ready, framework| {
                Box::pin(async move {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(())
                })
            },
        )
        .build();

    let intents = GatewayIntents::non_privileged();

    let token = get_secret(&SecretId::Discord)
        .await?
        .ok_or_else(|| anyhow!("Failed to fetch Discord token!"))?;

    let mut client = ClientBuilder::new(String::from_utf8(token.data.to_vec())?, intents)
        .framework(framework)
        .await?;

    client.start().await?;

    Ok(())
}
