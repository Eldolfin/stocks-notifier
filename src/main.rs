use anyhow::Context;
use clap::Parser;
use log::info;
use teloxide::{payloads::SendMessageSetters, prelude::*};

mod report;

#[derive(Parser)]
struct Args {
    /// Don't send the report at the end, print it instead
    #[arg(short = 'n', long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    env_logger::init();
    info!("Initializing...");
    let conf = report::EnvConfig::new().context("Could not parse config from environment")?;
    let bot = Bot::from_env();

    info!("Fetching stocks prices from yahoo finance...");
    let report = report::Report::fetch_now(&conf)
        .await
        .context("Could not fetch stocks report")?;

    info!("Sending report on telegram...");
    if !report.week_losers.is_empty() {
        let message = report.to_formatted_message();
        if args.dry_run {
            info!("Dry run mode: report that would have been sent:");
            println!("{}", message);
        } else {
            bot.send_message(conf.chat_id, message)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await
                .context("Could not send message")?;
        }
    } else {
        info!("No alerts to send. Skipping...");
    }
    Ok(())
}
