use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
pub struct Args {
    /// Directory to scan (default: home directory)
    #[arg(default_value = "")]
    pub dir: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Max scan depth
    #[arg(long, default_value = "5")]
    pub depth: usize,
}

#[derive(Serialize)]
struct RepoInfo {
    name: String,
    path: String,
    branch: Option<String>,
    remotes: Vec<RemoteInfo>,
    is_dirty: bool,
}

#[derive(Serialize)]
struct RemoteInfo {
    name: String,
    url: String,
}

pub fn run(args: &Args) -> Result<()> {
    let scan_dir = if args.dir.is_empty() {
        dirs::home_dir().context("cannot determine home directory")?
    } else {
        let p = PathBuf::from(&args.dir);
        if p.as_os_str().len() == 2 && p.as_os_str().to_string_lossy().ends_with(':') {
            PathBuf::from(format!("{}\\", args.dir))
        } else {
            p
        }
    };

    if !scan_dir.exists() {
        anyhow::bail!("directory does not exist: {}", scan_dir.display());
    }

    let repos = scan_git_repos(&scan_dir, args.depth)?;

    if repos.is_empty() {
        println!("No git repositories found in {}", scan_dir.display());
        return Ok(());
    }

    if args.json {
        let json = serde_json::to_string_pretty(&repos)?;
        println!("{json}");
    } else {
        for repo in &repos {
            println!("{}", repo.name);
            println!("  path:    {}", repo.path);
            if let Some(ref branch) = repo.branch {
                println!("  branch:  {branch}");
            }
            for remote in &repo.remotes {
                println!("  remote:  {} -> {}", remote.name, remote.url);
            }
            if repo.is_dirty {
                println!("  status:  dirty (uncommitted changes)");
            }
            println!();
        }
        println!("Found {} repositories", repos.len());
    }

    Ok(())
}

fn scan_git_repos(root: &Path, max_depth: usize) -> Result<Vec<RepoInfo>> {
    let mut repos = Vec::new();

    let mut walker = WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                if name.starts_with('$')
                    || name.starts_with('.')
                    || matches!(
                        name.as_ref(),
                        "node_modules"
                            | "target"
                            | ".cargo"
                            | ".cache"
                            | "AppData"
                            | "Windows"
                            | "ProgramData"
                            | "System Volume Information"
                            | "Recovery"
                            | "pagefile.sys"
                    )
                {
                    return false;
                }
            }
            true
        });

    while let Some(entry) = walker.next() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_dir() {
            continue;
        }

        let git_dir = entry.path().join(".git");
        if !git_dir.exists() {
            continue;
        }

        if let Ok(repo) = git2::Repository::open(entry.path()) {
            let name = entry
                .path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let branch = repo
                .head()
                .ok()
                .and_then(|h| h.shorthand().map(String::from));

            let remotes = extract_remotes(&repo);
            let is_dirty = is_dirty(&repo);

            repos.push(RepoInfo {
                name,
                path: entry.path().to_string_lossy().to_string(),
                branch,
                remotes,
                is_dirty,
            });
        }
    }

    Ok(repos)
}

fn extract_remotes(repo: &git2::Repository) -> Vec<RemoteInfo> {
    let Ok(remotes) = repo.remotes() else {
        return Vec::new();
    };

    remotes
        .iter()
        .flatten()
        .filter_map(|name| {
            let url = repo.find_remote(name).ok()?.url().map(String::from)?;
            Some(RemoteInfo {
                name: name.to_string(),
                url,
            })
        })
        .collect()
}

fn is_dirty(repo: &git2::Repository) -> bool {
    let Ok(statuses) = repo.statuses(None) else {
        return false;
    };

    statuses.iter().any(|s| {
        let s = s.status();
        s.is_wt_new()
            || s.is_wt_modified()
            || s.is_wt_deleted()
            || s.is_wt_renamed()
            || s.is_index_new()
            || s.is_index_modified()
            || s.is_index_deleted()
            || s.is_index_renamed()
    })
}
