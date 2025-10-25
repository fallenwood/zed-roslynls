WIP
---

# Zed C#

A [C#](https://learn.microsoft.com/en-us/dotnet/csharp/) extension for [Zed](https://zed.dev).

fork of https://github.com/zed-extensions/csharp/ and https://github.com/Digni/csharp/ , debugger inspired by https://github.dev/marcptrs/csharp_roslyn

## Development

To develop this extension, see the [Developing Extensions](https://zed.dev/docs/extensions/developing-extensions) section of the Zed docs.

## Example settings

```json
  "lsp": {
    "roslyn": {
      "settings": {
        "roslynls_path": "/home/vbox/workspace/zed-roslynls/wrapper/output/roslynls"
      }
    }
  }
```

## TODOs
- [x] Add OnReady hook to send `project/open` and `solution/open` to lsp
- [x] Diagnostic
- [x] Debugger
- [ ] Automatic package restore
