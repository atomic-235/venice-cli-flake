# venice-cli-flake

[Venice AI](https://venice.ai) CLI, built from source with `rustPlatform`.

## Commands

```text
venice balance
venice models [--type image|text|audio|music] [--verbose]
venice dashboard [--type T] [--in M] [--out M] [--vendor V] [--tag T] [--privacy P] [--sort F] [--desc] [--search S]
venice image <prompt> [--model ID] [--width N] [--height N] [--aspect-ratio R] [--resolution R] [--out PATH] [--no-open]
venice vision [IMAGE] [--prompt TEXT] [--model ID] [--list-models]
venice video [URL] [--prompt TEXT] [--model ID] [--list-models]
venice tts <text> [--voice NAME] [--model ID] [--format mp3|wav|opus|aac|flac] [--speed N] [--out PATH] [--no-play]
venice search <query> [--provider brave|google|exa] [--limit 1-20] [--json]
venice scrape <url> [--format markdown|html|text] [--out PATH] [--json]
venice xsearch <query> [--model grok-4-20] [--json]
```

`dashboard` is an interactive ratatui table: `q` quit, `/` search, `j/k` move,
`b` cycle filter field, `h/l` change its value, `s` cycle sort, `r` reverse,
`d` toggle details.

## Usage

```bash
nix run github:atomic-235/venice-cli-flake -- search "NixOS 26.11"
```

```nix
{
  inputs.venice-cli-flake.url = "github:atomic-235/venice-cli-flake";
  inputs.venice-cli-flake.inputs.nixpkgs.follows = "nixpkgs";
}
```

```nix
nixpkgs.overlays = [ inputs.venice-cli-flake.overlays.default ];
# pkgs.venice
```

## Runtime

The CLI reads `VENICE_API_KEY` and the standard proxy variables at runtime.
For x402 wallet auth, provide `VENICE_SIWX_TOKEN` instead of an API key.
No credentials are stored in this repository.
