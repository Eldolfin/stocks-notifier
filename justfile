set dotenv-load := true
export RUST_LOG := "debug"

# Display available commands
help:
    just --list

# Run the program every time a file changes
watch:
    git ls-files | entr -c just run

# Build & run the program
run:
    cargo run
