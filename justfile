set dotenv-load := true
set positional-arguments
export RUST_LOG := "debug"

# Display available commands
help:
    just --list

# Run the program every time a file changes
watch:
    git ls-files | entr -c just run

# Build & run the program
@run *args='':
    cargo run -- $@

# Build & run the program saving logs to ./logs/
run-logs:
    @mkdir -p logs
    just run 2>&1 | tee "logs/$(date --rfc-3339=seconds)"

# Deploy to my server
deploy:
    git push --force-with-lease deploy
    ssh ubuntu@search.eldolfin.top "cd stocks-notifier && git reset --hard"
