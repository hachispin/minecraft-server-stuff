use std::net::Ipv4Addr;

use anyhow::{Result, bail};
use google_cloud_compute_v1::{client::Instances, model::instance::Status};
use google_cloud_lro::Poller;
use google_cloud_secretmanager_v1::client::SecretManagerService;

const PROJECT: &str = "project-c863d0a5-25e6-435f-8b4";
const NAME: &str = "minecraft-server";
const ZONE: &str = "europe-west1-c";
const DESEC_UPDATE: &str = "https://update.dedyn.io/";
/// Must be hostname.
const DESEC_DNS: &str = "hachispin.dedyn.io";

/// I'm lazy.
macro_rules! set_fields {
    ($inst:expr) => {
        $inst.set_project(PROJECT).set_zone(ZONE).set_instance(NAME)
    };
}

/// Available secrets.
pub enum SecretId {
    Discord,
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

pub struct Controller {
    pub instances_client: Instances,
    pub secret_manager: SecretManagerService,
}

impl Controller {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            instances_client: Instances::builder().build().await?,
            secret_manager: SecretManagerService::builder().build().await?,
        })
    }

    /// Returns the specified secret's payload.
    ///
    /// This also stringifies the payload, because raw bytes are rarely useful.
    ///
    /// # Errors
    ///
    /// - If no payload is found for `secret_id`
    /// - Other typical stuff
    pub async fn get_secret(&self, secret_id: SecretId) -> Result<String> {
        let secret_id = secret_id.as_str();

        let response = self
            .secret_manager
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
    pub async fn get_status(&self) -> Result<Option<Status>> {
        let response = set_fields!(self.instances_client.get()).send().await?;

        Ok(response.status)
    }

    /// Gets the current external IPv4 address of the Minecraft VM.
    ///
    /// Returns the first if there are multiple. There shouldn't be multiple though.
    async fn get_ipv4(&self) -> Result<Option<Ipv4Addr>> {
        let response = set_fields!(self.instances_client.get()).send().await?;

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
    async fn set_dns_ipv4(&self, ipv4: Ipv4Addr) -> Result<()> {
        let token = self.get_secret(SecretId::DeSec).await?;
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

    /// Starts the VM. Polls until done in order to set DNS.
    pub async fn start_vm(&self) -> Result<()> {
        set_fields!(self.instances_client.start())
            .poller()
            .until_done()
            .await?;

        let Some(ip) = self.get_ipv4().await? else {
            bail!("VM has no external IPv4 address!?");
        };

        self.set_dns_ipv4(ip).await?;

        Ok(())
    }

    /// Stops the VM. **Does not poll until done**.
    pub async fn stop_vm(&self) -> Result<()> {
        set_fields!(self.instances_client.stop()).send().await?;

        Ok(())
    }
}
