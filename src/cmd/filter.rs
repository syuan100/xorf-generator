use crate::{
    cmd::{open_output_file, print_json},
    Filter, Manifest, PublicKeyManifest,
};
use anyhow::{Context, Result};
use helium_crypto::PublicKey;
use serde_json::json;
use std::{io::Write, path::PathBuf};

#[derive(clap::Args, Debug)]
pub struct Cmd {
    #[command(subcommand)]
    pub cmd: FilterCommand,
}

impl Cmd {
    pub fn run(&self) -> Result<()> {
        self.cmd.run()
    }
}

/// Commands on filters
#[derive(clap::Subcommand, Debug)]
pub enum FilterCommand {
    Generate(Generate),
    Contains(Contains),
    Verify(Verify),
    Info(Info),
}

impl FilterCommand {
    pub fn run(&self) -> Result<()> {
        match self {
            Self::Generate(cmd) => cmd.run(),
            Self::Contains(cmd) => cmd.run(),
            Self::Verify(cmd) => cmd.run(),
            Self::Info(cmd) => cmd.run(),
        }
    }
}

/// Check if a given filter file contains a given public key or edge.
#[derive(clap::Args, Debug)]
pub struct Contains {
    /// The input file to generate a filter for
    #[arg(long, short, default_value = "filter.bin")]
    input: PathBuf,
    /// The public key to check
    key: PublicKey,
    /// The publc key of the target of an edge to check
    target: Option<PublicKey>,
}

impl Contains {
    pub fn run(&self) -> Result<()> {
        let filter = Filter::from_path(&self.input)
            .context(format!("reading filter {}", self.input.display()))?;
        let in_filter = if let Some(target) = &self.target {
            filter.contains_edge(&self.key, target)
        } else {
            filter.contains(&self.key)
        };
        let json = json!({
            "address":  self.key.to_string(),
            "in_filter": in_filter,
        });
        print_json(&json)
    }
}

/// Verifies a given filter against the given multisig public key
#[derive(clap::Args, Debug)]
pub struct Verify {
    /// The input file to verify the signature for
    #[arg(long, short, default_value = "filter.bin")]
    input: PathBuf,
    /// The public key to use for verification
    #[arg(long, short, default_value = "public_key.json")]
    key: PathBuf,
}

impl Verify {
    pub fn run(&self) -> Result<()> {
        let filter = Filter::from_path(&self.input)
            .context(format!("reading filter {}", self.input.display()))?;
        let key_manifest = PublicKeyManifest::from_path(&self.key)
            .context(format!("reading public key {}", self.key.display()))?;
        let key = key_manifest.public_key()?;
        let verified = filter.verify(&key).is_ok();
        if !verified {
            anyhow::bail!("Filter does not verify");
        }
        print_verified(&key, verified)
    }
}

/// Generate a binary filter for the hotspots listed in the given file.
///
/// This converts a generated data binary, with a given multisig public key and
/// manifest and generates a signed binary xor filter (a binary fuse with 32 bit
/// fingerprints to be precise).
#[derive(Debug, clap::Args)]
pub struct Generate {
    /// The data file with signing data, generated by the manifest command, to
    /// generate a filter for
    #[arg(long, short, default_value = "data.bin")]
    data: PathBuf,
    /// The public key file to use
    #[arg(long, short, default_value = "public_key.json")]
    key: PathBuf,

    /// The file to write the resulting binary filter to
    #[arg(long, short, default_value = "filter.bin")]
    output: PathBuf,

    /// The path for the signature manifet to use
    #[arg(long, short, default_value = "manifest.json")]
    manifest: PathBuf,
}

impl Generate {
    pub fn run(&self) -> Result<()> {
        let manifest = Manifest::from_path(&self.manifest)
            .context(format!("reading manifest {}", self.manifest.display()))?;
        let key_manifest = PublicKeyManifest::from_path(&self.key)
            .context(format!("reading public key {}", self.key.display()))?;
        let key = key_manifest.public_key()?;

        let mut filter = Filter::from_signing_path(&self.data)?;
        filter.signature = manifest.sign(&key_manifest)?;
        filter.serial = manifest.serial;
        let filter_bytes = filter.to_bytes()?;
        let mut file = open_output_file(&self.output, false)?;
        file.write_all(&filter_bytes)?;

        let verified = filter.verify(&key).is_ok();
        if !verified {
            anyhow::bail!("Filter does not verify");
        }
        print_verified(&key, verified)
    }
}

/// Displays filter information for a given filter
#[derive(clap::Args, Debug)]
pub struct Info {
    /// The input file to generate a filter for
    #[arg(long, short, default_value = "filter.bin")]
    input: PathBuf,
}

impl Info {
    pub fn run(&self) -> Result<()> {
        let filter = Filter::from_path(&self.input)
            .context(format!("reading filter {}", self.input.display()))?;

        let mut json = serde_json::to_value(&filter)?;
        json["fingerprints"] = filter.filter.len().into();
        print_json(&json)
    }
}

fn print_verified(public_key: &PublicKey, verified: bool) -> Result<()> {
    let json = json!({
        "address":  public_key.to_string(),
        "verified": verified,
    });
    print_json(&json)
}
