# Stocks Notifier

A simple program that sends alerts when a watched stock drops by a specified
percentage within a short time frame.

![Message example](https://github.com/user-attachments/assets/d9f9f1be-b075-4d4d-bfc0-f5d694caf3e8)

(The thresholds are purposefully low here for demonstration)

## Setup

Copy `.env.example` to `.env` and fill it in

## Usage

To run it once, use `just run`.

You can then add this command to a cron job to run every day. For example:

```cron
# m h  dom mon dow   command
  0 10 *   *   *     bash -lc "just --justfile ~/stocks-notifier/justfile run-logs"
```

This will run the bot every day at 10AM
