//! `ai-memory move-session` — thin HTTP client for moving one session (or
//! every session of a project) to another project.

use anyhow::Result;
use serde::Serialize;

use crate::cli::MoveSessionArgs;
use crate::config::Config;
use crate::http_client::{ServerEndpoint, post_json};

/// Request sent to `POST /admin/move-session`. Exactly one of `session_id`
/// / `from_project` is set.
#[derive(Serialize)]
struct MoveSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<String>,
    project: String,
    pages: String,
    confirm: bool,
    force: bool,
    create: bool,
}

/// Run the `move-session` subcommand.
///
/// Without `--confirm` the server runs the move inside a rolled-back
/// transaction and this prints what would change plus the exact command to
/// apply it. With `--confirm` it prints the report.
///
/// # Errors
/// Returns an error when the server is unreachable or answers non-2xx (404
/// unknown session/target, 409 guard or page collision, 422 same scope).
pub async fn run(config: &Config, args: MoveSessionArgs) -> Result<()> {
    let endpoint = ServerEndpoint::from_config_resolving_auth(config).await;

    // The batch source is a scope like any other command's: marker, else
    // literal. The destination stays literal.
    let (from_workspace, from_project) = match args.from_project.as_deref() {
        Some(project) => {
            let (ws, proj) =
                super::resolve_scope(config, args.from_workspace.as_deref(), Some(project))?;
            (Some(ws), Some(proj))
        }
        None => (None, None),
    };
    let request = MoveSessionRequest {
        session_id: args.session_id.map(|id| id.to_string()),
        from_workspace,
        from_project,
        workspace: args.to_workspace.clone(),
        project: args.to.clone(),
        pages: args.pages.clone(),
        confirm: args.confirm,
        force: args.force,
        create: args.create,
    };
    let report: serde_json::Value = post_json(&endpoint, "/admin/move-session", &request).await?;

    if request.session_id.is_some() {
        print_session_report(&report, args.confirm);
    } else {
        print_batch_report(&report, args.confirm);
    }
    if !args.confirm {
        println!(
            "\n(dry run; nothing was written)\nRe-run with --confirm to apply:\n\n  {}",
            apply_command(&args)
        );
    }
    Ok(())
}

fn scope(label: &serde_json::Value) -> String {
    format!(
        "{}/{}",
        label["workspace"].as_str().unwrap_or("?"),
        label["project"].as_str().unwrap_or("?")
    )
}

fn print_session_report(report: &serde_json::Value, applied: bool) {
    let verb = if applied { "Moved" } else { "Would move" };
    println!(
        "{verb} session {} from {} to {}:",
        report["session_id"].as_str().unwrap_or("?"),
        scope(&report["from"]),
        scope(&report["to"]),
    );
    print_counts(
        &report["summary"],
        report["page"].as_str().unwrap_or("none"),
    );
    if let Some(warning) = report["cwd_warning"].as_str() {
        println!("Warning: {warning}");
    }
    if let Some(oid) = report["checkpoint"].as_str() {
        println!("Checkpoint: {oid}");
    }
}

fn print_batch_report(report: &serde_json::Value, applied: bool) {
    let verb = if applied { "Moved" } else { "Would move" };
    let total = report["total"].as_u64().unwrap_or(0);
    let moved = report["moved"].as_u64().unwrap_or(0);
    println!(
        "{verb} {moved} of {total} session(s) from {} to {}:",
        scope(&report["from"]),
        scope(&report["to"]),
    );
    if let Some(sessions) = report["sessions"].as_array() {
        println!(
            "{:<38} {:>6} {:>8} {:>5} {:<12} cwd",
            "session_id", "obs", "handoffs", "jobs", "page"
        );
        println!("{}", "-".repeat(90));
        for s in sessions {
            println!(
                "{:<38} {:>6} {:>8} {:>5} {:<12} {}",
                s["session_id"].as_str().unwrap_or("?"),
                s["summary"]["observations"].as_u64().unwrap_or(0),
                s["summary"]["handoffs"].as_u64().unwrap_or(0),
                s["summary"]["consolidation_jobs"].as_u64().unwrap_or(0),
                s["page"].as_str().unwrap_or("none"),
                s["cwd"].as_str().unwrap_or(""),
            );
        }
        let warned = sessions
            .iter()
            .filter(|s| s["cwd_warning"].is_string())
            .count();
        if warned > 0 {
            println!(
                "Warning: {warned} session(s) have a cwd whose basename is not the destination \
                 project; new sessions from those directories still resolve by basename unless \
                 a .ai-memory.toml marker pins the project."
            );
        }
    }
    if let Some(oid) = report["checkpoint"].as_str() {
        println!("Checkpoint: {oid}");
    }
}

fn print_counts(summary: &serde_json::Value, page: &str) {
    let n = |key: &str| summary[key].as_u64().unwrap_or(0);
    println!("  observations:        {}", n("observations"));
    println!("  handoffs:            {}", n("handoffs"));
    println!("  consolidation jobs:  {}", n("consolidation_jobs"));
    println!("  auto-improve runs:   {}", n("auto_improve_runs"));
    println!("  auto-improve claims: {}", n("auto_improve_claims"));
    println!(
        "  page:                {page} ({} version(s) moved, {} retired)",
        n("page_versions_moved"),
        n("pages_regenerated")
    );
}

/// The exact command that applies what the dry run showed.
fn apply_command(args: &MoveSessionArgs) -> String {
    let mut cmd = String::from("ai-memory move-session");
    if let Some(id) = args.session_id {
        cmd.push(' ');
        cmd.push_str(&id.to_string());
    }
    if let Some(project) = &args.from_project {
        cmd.push_str(&format!(" --from-project {project}"));
    }
    if let Some(ws) = &args.from_workspace {
        cmd.push_str(&format!(" --from-workspace {ws}"));
    }
    cmd.push_str(&format!(" --to {}", args.to));
    if let Some(ws) = &args.to_workspace {
        cmd.push_str(&format!(" --to-workspace {ws}"));
    }
    if args.pages != "move" {
        cmd.push_str(&format!(" --pages {}", args.pages));
    }
    if args.force {
        cmd.push_str(" --force");
    }
    if args.create {
        cmd.push_str(" --create");
    }
    cmd.push_str(" --confirm");
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(session_id: Option<&str>, from_project: Option<&str>) -> MoveSessionArgs {
        MoveSessionArgs {
            session_id: session_id.map(|s| s.parse().unwrap()),
            from_project: from_project.map(str::to_string),
            from_workspace: None,
            to: "target".into(),
            to_workspace: None,
            pages: "move".into(),
            confirm: false,
            force: false,
            create: false,
        }
    }

    #[test]
    fn apply_command_repeats_the_single_form_with_confirm() {
        let id = "0192b6a1-4c2e-7d3f-8a5b-1234567890ab";
        let mut a = args(Some(id), None);
        a.pages = "regenerate".into();
        a.to_workspace = Some("other".into());
        assert_eq!(
            apply_command(&a),
            format!(
                "ai-memory move-session {id} --to target --to-workspace other \
                 --pages regenerate --confirm"
            )
        );
    }

    #[test]
    fn apply_command_repeats_the_batch_form_and_flags() {
        let mut a = args(None, Some("ghost"));
        a.from_workspace = Some("default".into());
        a.force = true;
        a.create = true;
        assert_eq!(
            apply_command(&a),
            "ai-memory move-session --from-project ghost --from-workspace default --to target \
             --force --create --confirm"
        );
    }
}
