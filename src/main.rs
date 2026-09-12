#![warn(clippy::pedantic)]
#![allow(clippy::enum_glob_use)]

use std::net::Ipv4Addr;

use anyhow::{Result, anyhow, bail};
use google_cloud_compute_v1::{client::Instances, model::instance::Status};
use google_cloud_secretmanager_v1::{client::SecretManagerService, model::SecretPayload};
use poise::{Framework, FrameworkOptions, serenity_prelude::*};

type Context<'a> = poise::Context<'a, (), anyhow::Error>;

const PROJECT: &str = "project-c863d0a5-25e6-435f-8b4";
const NAME: &str = "minecraft-server";
const ZONE: &str = "europe-west1-c";
const DESEC_UPDATE: &str = "https://update.dedyn.io/";
const DESEC_DNS: &str = "https://hachispin.dedyn.io/";

/// Available secrets.
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

/// Returns the specified secret's payload.
///
/// This also stringifies the payload, because raw bytes are rarely useful.
///
/// # Errors
///
/// - If no payload is found for `secret_id`
/// - Other typical stuff
async fn get_secret(secret_id: &SecretId) -> Result<String> {
    let secret_id = secret_id.as_str();
    let service = SecretManagerService::builder().build().await?;

    let response = service
        .access_secret_version()
        .set_name(format!(
            "projects/{PROJECT}/secrets/{secret_id}/versions/latest"
        ))
        .send()
        .await?;

    let Some(payload) = response.payload else {
        bail!("No payload found for {secret_id}");
    };

    Ok(String::from_utf8(payload.data.to_vec())?)
}

/// Gets the status for the Minecraft VM.
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

/// Gets the current external IPv4 address of the Minecraft VM.
///
/// Returns the first if there are multiple. There shouldn't be multiple though.
async fn get_ipv4() -> Result<Option<Ipv4Addr>> {
    let client = Instances::builder().build().await?;

    let response = client
        .get()
        .set_project(PROJECT)
        .set_zone(ZONE)
        .set_instance(NAME)
        .send()
        .await?;

    let ip_string = response
        .network_interfaces
        .iter()
        .flat_map(|ni| &ni.access_configs)
        .find_map(|cfg| cfg.nat_ip.as_ref());

    Ok(ip_string.and_then(|s| s.parse::<Ipv4Addr>().ok()))
}

/// Modifies the deSEC DNS to point to the current external IPv4 address of the Minecraft VM.
///
/// Should be run every time the Minecraft VM starts.
async fn set_dns_ipv4(ipv4: Ipv4Addr) -> Result<()> {
    let token = get_secret(&SecretId::DeSec).await?;
    let client = reqwest::Client::new();

    let response = client
        .get(DESEC_UPDATE)
        .query(&[("hostname", DESEC_DNS), ("myipv4", &ipv4.to_string())])
        .header("Authorization", format!("Token {token}"))
        .send()
        .await?;

    response.error_for_status()?;

    Ok(())
}

/// Wrapper around [`get_status`] for Discord.
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

    let token = get_secret(&SecretId::Discord).await?;
    let intents = GatewayIntents::non_privileged();

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

    let mut client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    client.start().await?;

    Ok(())
}
