# Sublist3r-rs

A simple passive subdomain enumeration tool written in Rust. Inspired by [Sublist3r](https://github.com/aboul3la/Sublist3r) and [subfinder](https://github.com/projectdiscovery/subfinder).

## Installation

### Using `cargo`

```bash
git clone https://github.com/hamnghi250699/sublist3r-rs.git
cd sublist3r-rs
cargo install --path .
```

## Usage

Basic usage (this will use all available search engines):

```bash
s7r -d example.com
```

With specific search engines:

```bash
s7r -d example.com -e crtsh -e virustotal

# or as comma-separated values
s7r -d example.com -e crtsh,virustotal
```

Enable verbose output:

```bash
s7r -d example.com -v
```

Generate shell completions:

```bash
s7r --completion bash  # or zsh, fish, etc.
```

### Full Help Message

```bash
Usage: s7r [OPTIONS]

Options:
  -d, --domain <DOMAIN>
          Domain name to enumerate it's subdomains

  -e, --engines <ENGINES>
          Specify a comma-separated list of search engines
          
          [possible values: alienvault, bing, crtsh, dnsdumpster, google, hackertarget, rapiddns, virustotal, yahoo]

  -v, --verbose
          Enable Verbosity and display results in realtime

  -c, --completion <COMPLETION>
          Generate completion for the given shell
          
          [possible values: bash, elvish, fish, powershell, zsh]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Author

Nghi Nguyen (<hamnghi250699@gmail.com>)
