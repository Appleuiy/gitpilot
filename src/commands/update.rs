use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Only check for updates without installing
    #[arg(long)]
    pub check: bool,
}

pub fn run(args: &Args, current_version: &str) -> Result<()> {
    let target = self_update::get_target().to_string();

    if args.check {
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner("Appleuiy")
            .repo_name("gitpilot")
            .build()?
            .fetch()?;

        println!("current: v{current_version}");
        if let Some(release) = releases.into_iter().next() {
            let latest_ver = release.version.trim_start_matches('v');
            println!("latest:  v{latest_ver}");
            if latest_ver != current_version {
                println!("update available!");
            } else {
                println!("already up to date.");
            }
        } else {
            println!("no releases found.");
        }
        return Ok(());
    }

    println!("updating gitpilot from v{current_version}...");
    let status = self_update::backends::github::Update::configure()
        .repo_owner("Appleuiy")
        .repo_name("gitpilot")
        .target(&target)
        .bin_name("gitpilot")
        .show_download_progress(true)
        .current_version(current_version)
        .build()?
        .update()?;

    println!("updated to v{}", status.version());
    Ok(())
}
