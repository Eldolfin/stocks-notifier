use anyhow::Context;
use log::{debug, info};
use teloxide::{payloads::SendMessageSetters, prelude::*, utils::markdown};
use yahoo_finance_api::{self as yahoo, Quote};

struct EnvConfig {
    chat_id: ChatId,
    watched_stocks: Vec<String>,
}

impl EnvConfig {
    fn new() -> anyhow::Result<Self> {
        fn var(name: &str) -> anyhow::Result<std::string::String> {
            std::env::var(name).with_context(|| format!("Missing configuration variable `{name}`"))
        }

        fn parsed_var<T: std::str::FromStr>(name: &str) -> anyhow::Result<T>
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
            watched_stocks,
        })
    }
}

struct AlertItem {
    ticker_full_name: String,
    ticker_name: String,
    delta: f64,
    ticker_before: Quote,
    ticker_now: Quote,
}

struct Report {
    bullish_crossovers: Vec<AlertItem>,
    bearish_crossovers: Vec<AlertItem>,
}

impl Report {
    async fn fetch_now(conf: &EnvConfig) -> anyhow::Result<Self> {
        let provider = yahoo::YahooConnector::new().context("Error connecting to yahoo")?;
        let mut bullish_crossovers = Vec::new();
        let mut bearish_crossovers = Vec::new();
        for ticker in &conf.watched_stocks {
            let ticker_history = provider
                .get_quote_range(ticker, "1d", "3mo")
                .await
                .with_context(|| format!("Failed to retrieve data for {ticker}"))?;
            let ticker_meta = ticker_history
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
            let quotes = ticker_history
                .quotes()
                .with_context(|| format!("Failed to get quotes for {ticker}"))?;

            let closes: Vec<f64> = quotes.iter().map(|q| q.close).collect();
            let sma50 = closes.windows(50).map(|w| w.iter().sum::<f64>() / 50.0).collect::<Vec<f64>>();
            let sma14 = closes.windows(14).map(|w| w.iter().sum::<f64>() / 14.0).collect::<Vec<f64>>();

            if let (Some(last_sma50), Some(prev_sma50), Some(last_sma14), Some(prev_sma14)) = 
                (sma50.last(), sma50.iter().nth_back(1), sma14.last(), sma14.iter().nth_back(1))
            {
                let (last_sma50, prev_sma50, last_sma14, prev_sma14) = (*last_sma50, *prev_sma50, *last_sma14, *prev_sma14);

                // Bullish crossover: 14-day SMA crosses above 50-day SMA
                if prev_sma14 < prev_sma50 && last_sma14 > last_sma50 {
                    debug!("✅ Adding a bullish crossover alert for {ticker}");
                    bullish_crossovers.push(AlertItem {
                        ticker_full_name: ticker_full_name.clone(),
                        ticker_name: ticker.to_owned(),
                        delta: last_sma14 - last_sma50,
                        ticker_before: quotes.iter().nth_back(1).unwrap().to_owned(),
                        ticker_now: quotes.iter().last().unwrap().to_owned(),
                    });
                }

                // Bearish crossover: 14-day SMA crosses below 50-day SMA
                if prev_sma14 > prev_sma50 && last_sma14 < last_sma50 {
                    debug!("✅ Adding a bearish crossover alert for {ticker}");
                    bearish_crossovers.push(AlertItem {
                        ticker_full_name: ticker_full_name.clone(),
                        ticker_name: ticker.to_owned(),
                        delta: last_sma14 - last_sma50,
                        ticker_before: quotes.iter().nth_back(1).unwrap().to_owned(),
                        ticker_now: quotes.iter().last().unwrap().to_owned(),
                    });
                }
            }
        }
        bullish_crossovers.sort_by(|a, b| {
            a.delta
                .partial_cmp(&b.delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        bearish_crossovers.sort_by(|a, b| {
            a.delta
                .partial_cmp(&b.delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(Self {
            bullish_crossovers,
            bearish_crossovers,
        })
    }

    fn to_formatted_message(&self) -> String {
        let half_bar = markdown::escape(&"=".repeat(16));
        let bullish_crossovers = if self.bullish_crossovers.is_empty() {
            String::new()
        } else {
            format!(
                "\n{half_bar} Bullish Crossovers {half_bar}\n{}",
                Self::formatted_message_section(&self.bullish_crossovers)
            )
        };
        let bearish_crossovers = if self.bearish_crossovers.is_empty() {
            String::new()
        } else {
            format!(
                "\n{half_bar} Bearish Crossovers {half_bar}\n{}",
                Self::formatted_message_section(&self.bearish_crossovers)
            )
        };
        format!(
            r#"\n🚨 __Stocks alert__ 🚨\n{bullish_crossovers}\n{bearish_crossovers}\n"#
        )
    }

    fn formatted_message_section(companies: &[AlertItem]) -> String {
        let max_ticker_length = companies
            .iter()
            .map(|c| c.ticker_full_name.len() + c.ticker_name.len())
            .max()
            .unwrap_or(0);
        companies
            .iter()
            .map(|alert| {
                let name = markdown::escape(&format!(
                    "{ticker_name:<width$} ({ticker_full_name})",
                    ticker_name = alert.ticker_name,
                    ticker_full_name = alert.ticker_full_name,
                    width = max_ticker_length - alert.ticker_full_name.len()
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    info!("Initializing...");
    let conf = EnvConfig::new().context("Could not parse config from environment")?;
    let bot = Bot::from_env();

    info!("Fetching stocks prices from yahoo finance...");
    let report = Report::fetch_now(&conf)
        .await
        .context("Could not fetch stocks report")?;

    info!("Sending report on telegram...");
    if !report.bullish_crossovers.is_empty() || !report.bearish_crossovers.is_empty() {
        let message = report.to_formatted_message();
        bot.send_message(conf.chat_id, message)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await
            .context("Could not send message")?;
    } else {
        info!("No alerts to send. Skipping...");
    }
    Ok(())
}
