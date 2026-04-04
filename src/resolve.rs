use crate::config;

/// Resolve a partial project name to a full project name.
/// Exact match takes priority, then substring match.
/// Errors if zero or multiple projects match.
pub fn project(partial: &str) -> anyhow::Result<String> {
    let projects = config::list_projects()?;

    // Exact match — skip fuzzy
    if projects.iter().any(|p| p == partial) {
        return Ok(partial.to_string());
    }

    let matches: Vec<&String> = projects.iter().filter(|p| p.contains(partial)).collect();

    match matches.len() {
        0 => {
            if projects.is_empty() {
                anyhow::bail!("No projects configured.");
            }
            anyhow::bail!(
                "No project matching '{}'. Available: {}",
                partial,
                projects.join(", ")
            );
        }
        1 => Ok(matches[0].clone()),
        _ => {
            let names: Vec<&str> = matches.iter().map(|s| s.as_str()).collect();
            anyhow::bail!(
                "'{}' matches multiple projects: {}",
                partial,
                names.join(", ")
            );
        }
    }
}

/// Resolve a partial repo directory name within a project.
/// Exact match takes priority, then substring match.
/// Errors if zero or multiple repos match.
pub fn repo(project: &str, partial: &str) -> anyhow::Result<String> {
    let project_config = config::load_project(project)?;
    let repos: Vec<&str> = project_config
        .repos
        .iter()
        .map(|r| r.dir.as_str())
        .collect();

    // Exact match — skip fuzzy
    if repos.contains(&partial) {
        return Ok(partial.to_string());
    }

    let matches: Vec<&&str> = repos.iter().filter(|r| r.contains(partial)).collect();

    match matches.len() {
        0 => {
            anyhow::bail!(
                "No repo matching '{}' in project '{}'. Available: {}",
                partial,
                project,
                repos.join(", ")
            );
        }
        1 => Ok(matches[0].to_string()),
        _ => {
            let names: Vec<&str> = matches.iter().map(|s| **s).collect();
            anyhow::bail!(
                "'{}' matches multiple repos in project '{}': {}",
                partial,
                project,
                names.join(", ")
            );
        }
    }
}
