use crate::{
    config::{APP_NAME, STATE_FILE},
    ui::pkg::pkgpage::InstallType,
};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub install_type: Option<InstallType>,
}

pub fn get_state() -> Option<State> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME);
    let path = xdg_dirs.get_state_file(STATE_FILE)?;
    let raw_state = fs::read_to_string(path).ok()?;
    let state: State = toml::from_str(&raw_state).ok()?;

    Some(state)
}

pub fn save_state(state: State) -> Result<()> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME);
    let path = xdg_dirs
        .get_state_file(STATE_FILE)
        .ok_or_else(|| anyhow!("Can't get state file: {}", STATE_FILE))?;

    if !fs::exists(&path)? {
        xdg_dirs.place_state_file(STATE_FILE)?;
    }

    let raw_state = toml::to_string(&state)?;

    fs::write(path, raw_state)?;

    Ok(())
}

pub fn update_state<F>(f: F) -> Result<()>
where
    F: FnOnce(&mut State),
{
    let mut state = get_state().unwrap_or(State { install_type: None });

    f(&mut state);

    save_state(state)
}
