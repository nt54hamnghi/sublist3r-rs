fmt:
    cargo +nightly fmt

alias f := fmt

check:
    cargo clippy -- -D clippy:all -W clippy::pedantic

alias c := check

fix:
    cargo clippy --fix 

watch:
    # @cargo watch -cqx "r"
    @cargo watch -cqx "run --bin debug"

alias w := watch

install:
   @cargo install --path .
   @s7r --completion zsh > ./_s7r
   @sudo mv ./_s7r /usr/share/zsh/site-functions/_s7r

alias i := install
