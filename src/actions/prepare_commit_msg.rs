use anyhow::Result;
use clap::arg;
use clap::ValueEnum;
use colored::Colorize;

use clap::Args;
use strum_macros::Display;

use std::fs;

use std::path::PathBuf;

use crate::git;

use crate::help::print_help_openai_api_key;
use crate::llms::{llm_client::LlmClient, openai::OpenAIClient};
use crate::settings::ModelProvider;

use crate::settings::Settings;
use crate::summarize::SummarizationClient;
use crate::util::SplitPrefixInclusive;

use crate::llms::tester_foobar::FooBarClient;

/// Enum representing the possible commit message sources
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Display, ValueEnum, Default)]
enum CommitSource {
    #[clap(name = "")]
    #[default]
    Empty,
    Message,
    Template,
    Merge,
    Squash,
    Commit,
}

/// Arguments for the PrepareCommitMsg action
#[derive(Args, Debug)]
pub(crate) struct PrepareCommitMsgArgs {
    /// Name of the file that has the commit message
    #[arg(long)]
    commit_msg_file: PathBuf,

    /// Description of the commit message's source
    #[arg(long, value_enum)]
    commit_source: CommitSource,

    /// SHA1 hash of the commit being amended
    #[arg(long)]
    commit_sha: Option<String>,

    /// Debugging tool to mock git repo state
    #[arg(long)]
    git_diff_content: Option<PathBuf>,
}
fn get_llm_client(settings: &Settings) -> Box<dyn LlmClient> {
    match settings {
        Settings {
            model_provider: Some(ModelProvider::TesterFoobar),
            ..
        } => Box::new(FooBarClient::new().unwrap()),
        Settings {
            model_provider: Some(ModelProvider::OpenAI),
            openai: Some(openai),
            ..
        } => {
            let client = OpenAIClient::new(openai.to_owned());
            if let Err(_e) = client {
                print_help_openai_api_key();
                panic!("OpenAI API key not found in config or environment");
            }
            Box::new(client.unwrap())
        }
        _ => panic!("Could not load LLM Client from config!"),
    }
}

pub(crate) async fn main(settings: Settings, args: PrepareCommitMsgArgs) -> Result<()> {
    match args.commit_source {
        CommitSource::Empty | CommitSource::Message | CommitSource::Commit => {}
        _ => {
            println!(
                "🤖 Skipping gptcommit because the githook isn't set up for the \"{}\" commit mode.",
                args.commit_source
            );
            return Ok(());
        }
    };

    let client = get_llm_client(&settings);
    let summarization_client = SummarizationClient::new(settings.to_owned(), client)?;

    println!(
        "{}",
        "🤖 Let's ask OpenAI to summarize those diffs! 🚀"
            .green()
            .bold()
    );

    let output = if let Some(git_diff_output) = args.git_diff_content {
        fs::read_to_string(git_diff_output)?
    } else {
        git::get_diffs()?
    };

    let file_diffs = output.split_prefix_inclusive("\ndiff --git ");
    let commit_message = summarization_client.get_commit_message(file_diffs).await?;

    // prepend output to commit message
    let original_message: String = if args.commit_msg_file.is_file() {
        fs::read_to_string(&args.commit_msg_file)?
    } else {
        String::new()
    };
    let message_to_write = if original_message.is_empty() {
        commit_message
    } else {
        format!("{original_message}\n\n{commit_message}")
    };
    println!("{}", "🤖 OpenAI Summary:".green().bold());
    println!("{}", message_to_write);
    fs::write(&args.commit_msg_file, message_to_write)?;

    Ok(())
}
