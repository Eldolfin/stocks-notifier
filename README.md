# Stocks Notifier

A simple program that sends alerts when a watched stock drops by a specified
percentage within a short time frame.

## Setup

Copy `.env.example` to `.env` and fill it

## Usage

To run it once, use `just run`.

You can then add this command in a cron job to run every day. For example:

```cron
# m h  dom mon dow   command
  0 10 *   *   *     bash -lc "just --justfile ~/stocks-notifier/justfile run-logs"
```

this will run the bot every day at 10AM
