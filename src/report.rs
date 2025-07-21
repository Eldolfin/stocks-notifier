use anyhow::Context;
use log::{debug, warn};
use teloxide::{prelude::*, utils::markdown};
use tokio::task::JoinSet;
use yahoo_finance_api::{self as yahoo, Quote};
pub(crate) struct EnvConfig {
    pub(crate) chat_id: ChatId,
    pub(crate) week_delta_threshold: f64,
    pub(crate) day_delta_threshold: f64,
    pub(crate) watched_stocks: Vec<String>,
}

impl EnvConfig {
    pub(crate) fn new() -> anyhow::Result<Self> {
        pub(crate) fn var(name: &str) -> anyhow::Result<std::string::String> {
            std::env::var(name).with_context(|| format!("Missing configuration variable `{name}`"))
        }

        pub(crate) fn parsed_var<T: std::str::FromStr>(name: &str) -> anyhow::Result<T>
        where
            <T as std::str::FromStr>::Err: std::error::Error,
            <T as std::str::FromStr>::Err: std::marker::Send,
            <T as std::str::FromStr>::Err: std::marker::Sync,
            <T as std::str::FromStr>::Err: 'static,
        {
            var(name)?
                .parse()
                .with_context(|| format!("Failed to parse `{name}`"))
        }

        let watched_stocks = var("WATCHED_STOCKS")?
            .split(",")
            .map(str::to_owned)
            .collect();

        Ok(Self {
            chat_id: ChatId(parsed_var("TELEGRAM_CHAT_ID")?),
            week_delta_threshold: parsed_var("WEEK_DELTA_THRESHOLD")?,
            day_delta_threshold: parsed_var("DAY_DELTA_THRESHOLD")?,
            watched_stocks,
        })
    }
}

pub(crate) struct AlertItem {
    pub(crate) ticker_full_name: String,
    pub(crate) ticker_name: String,
    pub(crate) delta: f64,
    pub(crate) ticker_before: Quote,
    pub(crate) ticker_now: Quote,
}

pub(crate) struct Report {
    pub(crate) week_losers: Vec<AlertItem>,
    pub(crate) day_losers: Vec<AlertItem>,
}

impl Report {
    pub(crate) async fn fetch_now(conf: &EnvConfig) -> anyhow::Result<Self> {
        let mut week_losers = Vec::new();
        let mut day_losers = Vec::new();
        let tickers_futs: JoinSet<_> = conf
            .watched_stocks
            .to_owned()
            .into_iter()
            .map(|ticker| async move {
                let provider = yahoo::YahooConnector::new().context("Error connecting to yahoo")?;
                let ticker_data = provider
                    .get_quote_range(&ticker, "1d", "7d")
                    .await
                    .with_context(|| {
                        format!("Could not retrieve last 7 days for ticker {ticker}")
                    })?;
                Ok((ticker.to_owned(), ticker_data))
            })
            .collect();
        let tickers = tickers_futs
            .join_all()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        for (ticker, ticker_data) in tickers {
            let ticker_meta = ticker_data
                .metadata()
                .with_context(|| format!("Failed to get full name for {ticker}"))?;
            let ticker_full_name = ticker_meta
                .short_name
                .or(ticker_meta.long_name)
                .unwrap_or_else(|| "FULLNAME_MISSING".to_string())
                // cleanup name
                .replace("Inc.", "")
                .replace(",", "")
                .replace(" & Co.", "")
                .replace(" (The)", "")
                .replace("Corporation", "")
                .replace("Incorporated", "")
                .replace("Company", "")
                .trim()
                .to_string();
            let quotes = ticker_data
                .quotes()
                .with_context(|| format!("Failed to get quotes for {ticker}"))?;
            let Some(last_week) = quotes.iter().rev().nth(6) else {
                warn!("Skipping {ticker} as it's missing quotes[-7] (last week's price)");
                continue;
            };
            let Some(yesterday) = quotes.iter().rev().nth(1) else {
                // Impossible because it's not missing [-6]...
                warn!("Skipping {ticker} as it's missing quotes[-2] (yesterday's price)");
                continue;
            };
            let Some(now) = quotes.iter().last() else {
                warn!("Skipping {ticker} as it's missing quotes[-1] (today's price)");
                continue;
            };

            let delta_week = (now.close - last_week.open) / last_week.open * 100.;
            let delta_day = (now.close - yesterday.open) / yesterday.open * 100.;

            if delta_week < -conf.week_delta_threshold {
                debug!("✅ Adding a week alert for {ticker} ({delta_week:.2}%)");
                week_losers.push(AlertItem {
                    ticker_full_name: ticker_full_name.clone(),
                    ticker_name: ticker.to_owned(),
                    delta: delta_week,
                    ticker_before: last_week.to_owned(),
                    ticker_now: now.to_owned(),
                })
            } else {
                debug!("❌ NOT adding a week alert for {ticker} ({delta_week:.2}%)");
            }
            if delta_day < -conf.day_delta_threshold {
                debug!("✅ Adding a day alert for {ticker} ({delta_day:.2}%)");
                day_losers.push(AlertItem {
                    ticker_full_name: ticker_full_name.clone(),
                    ticker_name: ticker.to_owned(),
                    delta: delta_day,
                    ticker_before: yesterday.to_owned(),
                    ticker_now: now.to_owned(),
                })
            } else {
                debug!("❌ NOT adding a day alert for {ticker} ({delta_day:.2}%)");
            }
        }
        week_losers.sort_by(|a, b| {
            a.delta
                .partial_cmp(&b.delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        day_losers.sort_by(|a, b| {
            a.delta
                .partial_cmp(&b.delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(Self {
            week_losers,
            day_losers,
        })
    }

    pub(crate) fn to_formatted_message(&self) -> String {
        let half_bar = markdown::escape(&"=".repeat(16));
        let week_losers = if self.week_losers.is_empty() {
            String::new()
        } else {
            format!(
                "
{half_bar} Week losers {half_bar}
{}",
                Self::formatted_message_section(&self.week_losers)
            )
        };
        let day_losers = if self.day_losers.is_empty() {
            String::new()
        } else {
            format!(
                "
{half_bar} Day losers {half_bar}
{}",
                Self::formatted_message_section(&self.day_losers)
            )
        };
        format!(
            r#"
🚨 __Stocks alert__ 🚨
{week_losers}
{day_losers}
"#
        )
    }

    pub(crate) fn formatted_message_section(companies: &[AlertItem]) -> String {
        let max_ticker_length = companies
            .iter()
            .map(|c| c.ticker_full_name.len() + c.ticker_name.len())
            .max()
            .unwrap_or(0);
        companies
            .iter()
            .map(|alert| {
                let name = markdown::escape(&format!(
                    "{:width$} ({})",
                    alert.ticker_name,
                    alert.ticker_full_name,
                    width = max_ticker_length - alert.ticker_full_name.len(),
                ));
                let delta = markdown::escape(&format!("{:.2}%", alert.delta));
                let delta_details = markdown::escape(&format!(
                    "({:.2}$ -> {:.2}$)",
                    alert.ticker_before.open, alert.ticker_now.close
                ));
                format!("`{name}` *{delta}* _{delta_details}_",)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
