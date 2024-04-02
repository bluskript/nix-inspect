# nix-inspect
Interactive nix config viewer

```
nix run github:bluskript/nix-inspect
```

![[preview.mp4]]
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

- Legacy non-flake config support
- Loading arbitrary nix exprs defined by the user
