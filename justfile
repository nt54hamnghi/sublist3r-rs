fmt:
    cargo +nightly fmt

alias f := fmt

prep:
    cargo +nightly fmt
    git add .
    cargo clippy --fix --allow-staged -- -A clippy::correctness
    git reset

alias p := prep

watch:
    @cargo watch -cqx "run --bin debug"
    # @cargo watch -cqx "r"

alias w := watch

# zsh:
#    @sudo cp ./completion/zsh-completion /usr/share/zsh/site-functions/_sublist3r-rs

# install: zsh
#    @cargo install --path .

# alias i := install
