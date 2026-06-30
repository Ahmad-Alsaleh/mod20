# mod20

A small CLI for percent-decoding URLs and text, with an optional colorized breakdown of a URL's structure.

It reads input from an argument, from `-`, or from stdin, and can also pretty-print a URL into its components — automatically enriching values it recognizes as **nested URLs**, **JWTs**, or **base64**.

## Install

```bash
cargo install --path .
# or run directly during development
cargo run -- <args>
```

## Usage

```
mod20 [OPTIONS] [INPUT]
mod20 completions <SHELL>
```

`INPUT` is the URL or text to process. Use `-` or omit it to read from stdin.

### Options

- `-p, --pretty` — pretty-print the URL's structure with colors
- `--no-color` — disable colored output (also honored via the `NO_COLOR` env var)
- `-h, --help` — print help

## Examples

### Percent-decode some text

```bash
$ mod20 "hello%20world%21"
hello world!
```

```bash
$ mod20 "caf%C3%A9%20%E2%98%95"
café ☕
```

### Read from stdin

```bash
$ echo "name%3Dvalue%26x%3D1" | mod20
name=value&x=1

$ printf '%s' "a%2Fb%2Fc" | mod20 -
a/b/c
```

### Pretty-print a URL

```bash
$ mod20 --pretty "https://user:pass@example.com:8443/api/v1/search?q=hello%20world&tag=a&tag=b#section"
scheme: https
userinfo: user:pass
host: example.com
port: 8443
path:
  /api
  /v1
  /search
query:
  q = hello world
  tag (x2)
    - a
    - b
fragment: section
warning: embedded credentials in userinfo
```

### Pretty-print just a query string

If the input isn't an absolute URL, `--pretty` falls back to treating it as a query string:

```bash
$ mod20 --pretty "?from=%2Fhome&next=%2Fdashboard"
(not an absolute URL — showing query only)
query:
  from = /home
  next = /dashboard
```

### Automatic value enrichment

When pretty-printing, recognized values are decoded inline:

- **Nested URLs** (e.g. a `redirect=` parameter) are parsed and rendered as a sub-tree.
- **JWTs** have their header and payload decoded to pretty JSON.
- **base64** values that decode to printable text are shown.

```bash
$ mod20 --pretty "https://app.example.com/login?redirect=https%3A%2F%2Fexample.com%2Fdashboard%3Ftab%3Dbilling"
scheme: https
host: app.example.com
path:
  /login
query:
  redirect = https://example.com/dashboard?tab=billing
    ↳ nested url:
      scheme: https
      host: example.com
      path:
        /dashboard
      query:
        tab = billing
```

### Safety warnings

Pretty-print mode flags potentially risky URLs, including:

- `insecure scheme (http)`
- `embedded credentials in userinfo`
- `malformed percent-escape (not valid UTF-8)`

### Shell completions

Generate a completion script for your shell (`bash`, `zsh`, `fish`, `powershell`, `elvish`):

```bash
$ mod20 completions zsh > _mod20
```
