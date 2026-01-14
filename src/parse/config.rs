use anyhow::Result;
use nix_data_xinux::config::configfile::NixDataConfig;

pub fn getconfig() -> Option<NixDataConfig> {
    nix_data_xinux::config::configfile::getconfig().ok()
}

pub fn editconfig(config: NixDataConfig) -> Result<()> {
    nix_data_xinux::config::configfile::setuserconfig(config)?;
    Ok(())
}
