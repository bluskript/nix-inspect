# nix-inspect

A ranger-like TUI for inspecting your nixos config and other arbitrary nix expressions.

```
nix run github:bluskript/nix-inspect
```


https://github.com/bluskript/nix-inspect/assets/52386117/21cfc643-653c-43c8-abf1-d75c07f15b7f

### Motivation

A lot of the time my workflow using nixos would involve running a series of commands to find the final merged result of my config:
```
❯ : nix repl
nix-repl> :lf /etc/nixos
Added 18 variables.
nix-repl> nixosConfigurations.felys.config.home-manager.users.blusk.stylix.fonts.monospace.name
"Cozette"
```

`nix-inspect` aims to improve on this workflow by offering a interactive way to browse a config and offering quality of life features such as bookmarks and a path navigator mode to get where you need quickly.

### Features
- 🪡 Path navigator to quickly type in or paste a path which live updates as you type (.)
  - Supports tab completion!
- 🔍Fuzzy search in the current directory (Ctrl-F or /)
- 🔖 Bookmarks to save important nix paths, automatically populated with your current system and user (s)
- ⌨️ Vim keybindings (hjkl, ctl+u, ctrl+d)
- (planned) 🕑 Recently visited paths tab

### Usage

By default, `nix-inspect` will try to load your config where it is, by default this will be /etc/nixos if you are using flakes or the path in NIX_PATH if you are using legacy. If this behavior is not what you want, `nix-inspect` comes with some flags:

- `--expr` / `-e` - load an arbitrary expression. Example: `nix-inspect -e { a = 1; }`
- `--path` / `-p` - load a config at a specific path. Example: `nix-inspect -p /persist/etc/nixos`

### Key Bindings

| Key             | Behavior            |
| --------------- | ------------------- |
| q               | Exit                |
| h / left arrow  | Navigate up a level |
| j / down arrow  | Select lower item   |
| k / up arrow    | Select upper item   |
| l / right arrow | Enter selected item |
| f / "/"         | Search              |
| ctrl+d          | Half-Page Down      |
| ctrl+u          | Half-Page Up        |
| s               | Save bookmark       |
| .               | Path Navigator mode |


### Installation
This project has been added to nixpkgs, but there may have been changes not yet landed there. It is recommended to use nix-inspect as a flake like so:
```nix
{
  inputs = {
    nix-inspect.url = "github:bluskript/nix-inspect";
  };
}
```
and then reference it in your `environment.systemPackages`:
```nix
{inputs, ...}: {
  environment.systemPackages = [
    inputs.nix-inspect.packages.default
  ];
}
```
