use self_update::{backends::github, cargo_crate_version};

const OWNER: &str = "LuMiSxh";
const REPOSITORY: &str = "Thasia";
const BINARY: &str = "thasia-gpui";

pub fn latest_version() -> Result<Option<String>, self_update::errors::Error> {
    let releases = github::ReleaseList::configure()
        .repo_owner(OWNER)
        .repo_name(REPOSITORY)
        .build()?
        .fetch()?;
    let latest = releases
        .into_iter()
        .find(|release| self_update::version::bump_is_greater(cargo_crate_version!(), &release.version).unwrap_or(false));
    Ok(latest.map(|release| release.version))
}

pub fn install_latest() -> Result<String, self_update::errors::Error> {
    let status = github::Update::configure()
        .repo_owner(OWNER)
        .repo_name(REPOSITORY)
        .bin_name(BINARY)
        .show_download_progress(false)
        .show_output(false)
        .no_confirm(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;
    Ok(status.version().to_string())
}
