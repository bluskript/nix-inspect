# Changelog

### 0.1.2

- Hotfixed a dependency issue with ansi-to-tui which breaks builds
- Adjusted how paths are loaded by default to load cwd if /etc/nixos is unavailable

### 0.1.1

- Fixed bug with exploring items with dots in their name (services.nginx.virtualHosts."example.com".root etc.)
- Added support for moving forward and backward in search and path navigator mode (n = move forward, N = move backward)
- Fixed navigating up in the list shifting the entire view up
