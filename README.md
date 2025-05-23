# Stocks Notifier

A simple program that sends alerts when a watched stock drops by a specified
percentage within a short time frame.

![Message example](https://github.com/user-attachments/assets/d68c769e-00d2-4a3a-a3a5-eda024446e1d)


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
