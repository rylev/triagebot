//! Purpose: When opening a rollup PR, check for rolled up PRs that look particularly
//! dangerous and call them out.

use crate::{
    config::{MentionsConfig, MentionsPathConfig},
    db::issue_data::IssueData,
    github::{files_changed, IssuesAction, IssuesEvent},
    handlers::Context,
};
use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::path::Path;
use tracing as log;

const MENTIONS_KEY: &str = "mentions";

pub(super) struct MentionsInput {
    paths: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct RollupState {
    rollups: Vec<String>,
}

pub(super) async fn parse_input(
    ctx: &Context,
    event: &IssuesEvent,
    config: Option<&MentionsConfig>,
) -> Result<Option<MentionsInput>, String> {
    let config = match config {
        Some(config) => config,
        None => return Ok(None),
    };

    if !matches!(event.action, IssuesAction::Opened) {
        return Ok(None);
    }

    // Only ping on active rollup PRs.
    if !event.issue.title.starts_with("Rollup of") || event.issue.draft {
        return Ok(None);
    }

    let body = &event.issue.body;
    let prs = get_rolled_up_prs(body);

    let repo_url = event.issue.repository().url();
    for pr in prs {
        let diff = ctx.github.pr_diff(&repo_url, pr).await.unwrap();
        let files = files_changed(&diff);
        let dangerous_files = dangerous_files(files);
    }

    //     if let Some(diff) = event
    //         .body
    //         .diff(&ctx.github)
    //         .await
    //         .map_err(|e| {
    //             log::error!("failed to fetch diff: {:?}", e);
    //         })
    //         .unwrap_or_default()
    //     {
    //         let files = files_changed(&diff);
    //         let file_paths: Vec<_> = files.iter().map(|p| Path::new(p)).collect();
    //         let to_mention: Vec<_> = config
    //             .paths
    //             .iter()
    //             .filter(|(path, MentionsPathConfig { cc, .. })| {
    //                 let path = Path::new(path);
    //                 // Only mention matching paths.
    //                 let touches_relevant_files = file_paths.iter().any(|p| p.starts_with(path));
    //                 // Don't mention if only the author is in the list.
    //                 let pings_non_author = match &cc[..] {
    //                     [only_cc] => only_cc.trim_start_matches('@') != &event.issue.user.login,
    //                     _ => true,
    //                 };
    //                 touches_relevant_files && pings_non_author
    //             })
    //             .map(|(key, _mention)| key.to_string())
    //             .collect();
    //         if !to_mention.is_empty() {
    //             return Ok(Some(MentionsInput { paths: to_mention }));
    //         }
    //     }
    Ok(None)
}

fn dangerous_files(files: Vec<&str>) -> bool {
    todo!()
}

fn get_rolled_up_prs(body: &str) -> Vec<&str> {
    body.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .skip_while(|l| !l.starts_with("Successful merges:"))
        .skip(1)
        .take_while(|l| l.starts_with("- #"))
        .filter_map(|l| {
            let suffix = &l[3..];
            Some(&suffix[..suffix.find(' ')?])
        })
        .collect()
}

pub(super) async fn handle_input(
    ctx: &Context,
    config: &MentionsConfig,
    event: &IssuesEvent,
    input: MentionsInput,
) -> anyhow::Result<()> {
    // let mut client = ctx.db.get().await;
    // let mut state: IssueData<'_, RollupState> =
    //     IssueData::load(&mut client, &event.issue, MENTIONS_KEY).await?;
    // // Build the message to post to the issue.
    // let mut result = String::new();
    // for to_mention in &input.paths {
    //     if state.data.paths.iter().any(|p| p == to_mention) {
    //         // Avoid duplicate mentions.
    //         continue;
    //     }
    //     let MentionsPathConfig { message, cc } = &config.paths[to_mention];
    //     if !result.is_empty() {
    //         result.push_str("\n\n");
    //     }
    //     match message {
    //         Some(m) => result.push_str(m),
    //         None => write!(result, "Some changes occurred in {to_mention}").unwrap(),
    //     }
    //     if !cc.is_empty() {
    //         write!(result, "\n\ncc {}", cc.join(", ")).unwrap();
    //     }
    //     state.data.paths.push(to_mention.to_string());
    // }
    // if !result.is_empty() {
    //     event
    //         .issue
    //         .post_comment(&ctx.github, &result)
    //         .await
    //         .context("failed to post mentions comment")?;
    //     state.save().await?;
    // }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_get_rolled_up_prs() {
        let body = r#"Successful merges:

 - #101075 (Migrate rustc_codegen_gcc to SessionDiagnostics )
 - #102350 (Improve errors for incomplete functions in struct definitions)
 - #102481 (rustdoc: remove unneeded CSS `.rust-example-rendered { position }`)
 - #102491 (rustdoc: remove no-op source sidebar `opacity`)

Failed merges:


r? @ghost
@rustbot modify labels: rollup
<!-- homu-ignore:start -->
[Create a similar rollup](https://bors.rust-lang.org/queue/rust?prs=101075,102350,102481,102491,102499)
<!-- homu-ignore:end -->"#;
        let prs = get_rolled_up_prs(body);
        assert_eq!(prs, vec!["101075", "102350", "102481", "102491"]);
    }
}
