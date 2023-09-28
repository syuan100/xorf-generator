use crate::{
    cmd::{open_output_file, print_json},
    filter::Filter,
    manifest::{ManifestSignature, ManifestSignatureVerify},
    Descriptor, Manifest, PublicKeyManifest, Result,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use std::{io::Write, path::PathBuf};

#[derive(clap::Args, Debug)]
pub struct Cmd {
    #[command(subcommand)]
    pub cmd: ManifestCommand,
}

impl Cmd {
    pub fn run(&self) -> Result {
        self.cmd.run()
    }
}

/// Commands on manifests
#[derive(clap::Subcommand, Debug)]
pub enum ManifestCommand {
    Generate(Generate),
    Verify(Verify),
}

impl ManifestCommand {
    pub fn run(&self) -> Result {
        match self {
            Self::Generate(cmd) => cmd.run(),
            Self::Verify(cmd) => cmd.run(),
        }
    }
}

/// Generate a manifest for a given list of hotspots
///
/// This takes a a filename a descriptor of denied hotspots/edges and generates
/// a manifest file that can be used to add signatures to as well as binary file
/// which contains the data of the filter to sign.
#[derive(Debug, clap::Args)]

pub struct Generate {
    /// The descriptor file to generate a manifest for
    #[arg(long, short, default_value = "descriptor.json")]
    input: PathBuf,

    /// The file to write the filter signing data to
    #[arg(long, short, default_value = "data.bin")]
    data: PathBuf,

    /// The public key file to use
    #[arg(long, short, default_value = "public_key.json")]
    key: PathBuf,

    /// The file to write the resulting manifest file to
    #[arg(long, short, default_value = "manifest.json")]
    manifest: PathBuf,

    /// Whether to force overwrite an existing manifest file
    #[arg(long, short)]
    force: bool,

    /// The serial number for the filter
    #[arg(long, short)]
    serial: u32,
}

impl Generate {
    pub fn run(&self) -> Result {
        let descriptor = Descriptor::from_json(&self.input)?;
        let filter = Filter::from_descriptor(self.serial, &descriptor)?;
        let filter_hash = filter.hash()?;
        let key_manifest = PublicKeyManifest::from_path(&self.key)?;
        let signatures = key_manifest
            .public_keys
            .iter()
            .map(ManifestSignature::from)
            .collect();

        let mut manifest_file = open_output_file(&self.manifest, !self.force)?;
        let manifest = Manifest {
            serial: self.serial,
            hash: STANDARD.encode(filter_hash),
            signatures,
        };
        serde_json::to_writer_pretty(&mut manifest_file, &manifest)?;

        let mut data_file = open_output_file(&self.data, false)?;
        let signing_bytes = filter.to_signing_bytes()?;
        data_file.write_all(&signing_bytes)?;

        Ok(())
    }
}

/// Verify the manifest for a given datafile, public key and manifest file
///
/// This takes a a filename of a binary filter data file as well as the manifest
///  file and public multisig key, and displays whether the manifest verifies
#[derive(Debug, clap::Args)]

pub struct Verify {
    /// The file with the data bytes that were signed
    #[arg(long, short, default_value = "data.bin")]
    data: PathBuf,

    /// The public key file to use
    #[arg(long, short, default_value = "public_key.json")]
    key: PathBuf,

    /// The manifest file to verify
    #[arg(long, short, default_value = "manifest.json")]
    manifest: PathBuf,
}

impl Verify {
    pub fn run(&self) -> Result {
        let manifest = Manifest::from_path(&self.manifest)?;
        let manifest_hash = STANDARD.decode(&manifest.hash)?;
        let key_manifest = PublicKeyManifest::from_path(&self.key)?;
        let key = key_manifest.public_key()?;

        let filter = Filter::from_signing_path(&self.data)?;
        let filter_hash = filter.hash()?;
        let signing_bytes = filter.to_signing_bytes()?;

        let hash_verified = manifest_hash == filter_hash;
        let signtatures: Vec<ManifestSignatureVerify> = manifest
            .signatures
            .iter()
            .map(|signature| signature.verify(&signing_bytes))
            .collect();

        let json = json!({
            "signing_data": self.data,
            "hash": {
                "serial": manifest.serial,
                "hash": manifest.hash,
                "verified": hash_verified,
            },
            "public_key": key,
            "signatures": signtatures,
        });
        print_json(&json)
    }
}
