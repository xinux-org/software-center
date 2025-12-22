use anyhow::Result;
use nix_data::config::configfile::NixDataConfig;

pub fn getconfig() -> Option<NixDataConfig> {
    nix_data::config::configfile::getconfig().ok()
}

pub fn editconfig(config: NixDataConfig) -> Result<()> {
    nix_data::config::configfile::setuserconfig(config)?;
    Ok(())
}
