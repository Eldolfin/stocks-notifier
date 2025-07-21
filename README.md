[![Rust](https://github.com/Eldolfin/stocks-notifier/actions/workflows/rust.yml/badge.svg)](https://github.com/Eldolfin/stocks-notifier/actions/workflows/rust.yml)
# Stocks Notifier

A simple program that sends alerts when a watched stock drops by a specified
percentage within a short time frame.

![Message example](https://github.com/user-attachments/assets/d68c769e-00d2-4a3a-a3a5-eda024446e1d)

(The thresholds are purposefully low here for demonstration)

## Setup

Copy `.env.example` to `.env` and fill it in

## Usage

### Development test

To run it once, use `just run`. To run it every time the code change you can use
`just watch`.

### Scheduled on a server

You can then add this command to a cron job to run every day. For example:

```cron
# m h  dom mon dow   command
  0 10 *   *   *     bash -lc "just --justfile ~/stocks-notifier/justfile run-logs"
```

This will run the bot every day at 10AM

### Scheduled on github actions

- Fork this repo
- (optional) change the configuration in ./.github/workflows/cron.yml
- set `TELOXIDE_TOKEN` and `TELEGRAM_CHAT_ID` in your repo's secrets to your
  telegram bot api and the chat id you want notifications to be sent to
  - navigate to `https://github.com/<username>/stocks-notifier/settings/secrets/actions`
  - add the two variables as such
    <kbd>
      <img src="https://github.com/user-attachments/assets/1c2e50a5-ed78-4708-8e9e-9ef058963570">
    </kbd>

