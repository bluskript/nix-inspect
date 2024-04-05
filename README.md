# nix-inspect
Interactive nix config viewer

```
nix run github:bluskript/nix-inspect
```

https://github.com/bluskript/nix-inspect/assets/52386117/a18c6038-f954-451b-a8cd-26b30a197165

### Features
- 🪡 Path navigator to quickly type in or paste a path which live updates as you type
- 🔍Fuzzy search in the current directory
- 🔖 Bookmarks to save important nix paths, automatically populated with your current system and user
- ⌨️ Vim keybindings
- (planned) 🕑 Recently visited paths tab

### Installation
As of now the project remains unpackaged in `nixpkgs`, so the recommended installation method is through flakes:
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

### Known Issues / TODO

- Search / path navigator ui needs to be more visible
