# Aria Lang Editor Plugins

## Overview

Official and community-maintained editor plugins for Aria Lang, providing syntax highlighting, code completion, error checking, and more.

## Available Plugins

### 1. VSCode Extension
**Status**: In Development
**Features**:
- Syntax highlighting
- IntelliSense (auto-completion)
- Error diagnostics
- Code formatting
- Debugger integration
- Cyberpunk theme

**Installation**:
```bash
code --install-extension aria-lang.aria-vscode
```

[View in Marketplace](https://marketplace.visualstudio.com/items?itemName=aria-lang.aria-vscode)

---

### 2. Neovim Plugin
**Status**: Planned
**Features**:
- Tree-sitter grammar
- LSP integration
- Syntax highlighting
- Code actions
- Custom commands

**Installation** (via vim-plug):
```vim
Plug 'aria-lang/aria.nvim'
```

---

### 3. JetBrains Plugin
**Status**: Planned
**Features**:
- IntelliJ IDEA support
- Full IDE integration
- Refactoring tools
- Debugger
- Unit test runner

**Installation**:
Search for "Aria Lang" in JetBrains Marketplace

---

### 4. Sublime Text
**Status**: Community
**Features**:
- Basic syntax highlighting
- Build system integration

**Installation** (via Package Control):
```
Package Control: Install Package -> Aria Lang
```

---

## Language Server Protocol (LSP)

All plugins use the Aria Language Server for core functionality.

### Features Supported

✅ **Implemented**:
- Syntax highlighting
- Basic diagnostics

🚧 **In Progress**:
- Go to definition
- Find references
- Hover information
- Code completion

📅 **Planned**:
- Refactoring
- Code actions
- Semantic highlighting
- Inlay hints
- Call hierarchy

## Development

### Building Plugins

Each plugin directory contains its own build instructions:

```bash
# VSCode
cd plugins/vscode
npm install
npm run compile

# Neovim
cd plugins/neovim
# No build needed (Lua/VimScript)

# JetBrains
cd plugins/jetbrains
./gradlew buildPlugin
```

### Testing Locally

#### VSCode
```bash
cd plugins/vscode
npm run watch
# Press F5 in VSCode to launch Extension Development Host
```

#### Neovim
```bash
# Symlink to your config
ln -s $(pwd)/plugins/neovim ~/.config/nvim/pack/plugins/start/aria.nvim
```

## Contributing

Want to improve editor support? We welcome contributions!

### Priority Areas

1. **LSP Features**: Implement missing LSP capabilities
2. **Debugger**: DAP (Debug Adapter Protocol) support
3. **Themes**: Additional color schemes
4. **Snippets**: Code snippet collections
5. **Documentation**: Improve plugin docs

See [CONTRIBUTING.md](../community/CONTRIBUTING.md) for guidelines.

## Plugin Structure

### VSCode Extension

```
vscode/
├── package.json          # Extension manifest
├── tsconfig.json         # TypeScript config
├── src/
│   ├── extension.ts      # Main entry point
│   ├── client.ts         # LSP client
│   └── themes/
│       └── cyberpunk.json
├── syntaxes/
│   └── aria.tmLanguage.json
└── snippets/
    └── aria.json
```

### Neovim Plugin

```
neovim/
├── lua/
│   └── aria/
│       ├── init.lua      # Main plugin
│       ├── lsp.lua       # LSP config
│       └── treesitter.lua
├── queries/
│   └── aria/
│       ├── highlights.scm
│       └── injections.scm
└── ftdetect/
    └── aria.vim
```

### JetBrains Plugin

```
jetbrains/
├── build.gradle.kts
├── src/main/
│   ├── kotlin/
│   │   └── dev/arialang/plugin/
│   │       ├── AriaLanguage.kt
│   │       ├── AriaFileType.kt
│   │       └── highlighting/
│   └── resources/
│       ├── META-INF/plugin.xml
│       └── icons/
└── src/test/
```

## Resources

- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
- [VSCode Extension API](https://code.visualstudio.com/api)
- [Neovim LSP Guide](https://neovim.io/doc/user/lsp.html)
- [IntelliJ Platform SDK](https://plugins.jetbrains.com/docs/intellij/)

## Support Matrix

| Feature | VSCode | Neovim | JetBrains | Sublime |
|---------|--------|--------|-----------|---------|
| Syntax Highlighting | ✅ | ✅ | 🚧 | ✅ |
| Auto-completion | 🚧 | 🚧 | ❌ | ❌ |
| Error Checking | 🚧 | 🚧 | ❌ | ❌ |
| Go to Definition | ❌ | ❌ | ❌ | ❌ |
| Refactoring | ❌ | ❌ | ❌ | ❌ |
| Debugging | ❌ | ❌ | ❌ | ❌ |
| Formatting | ✅ | ❌ | ❌ | ❌ |

Legend: ✅ Available, 🚧 In Progress, ❌ Not Started

## Questions?

- Discord: #editor-plugins channel
- GitHub: Create an issue
- Email: plugins@aria-lang.dev

---

*"Edit with style in your favorite editor."*
