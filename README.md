# symseek

A simple utility to trace links recursively.

## Usage

```shell
symseek [OPTIONS] <TARGET>
```

- `TARGET`: target file or directory. If only a filename is specified, will also search in `PATH`.

## Detection

Current the following types of links are handled:

- symlinks
- nixpkgs wrappers
  - Heuristics: check if the file contains a nix store path with the same app name
