# Aria Playground

## Overview

The Aria Playground is a WASM-powered, in-browser code editor and execution environment for Aria Lang, featuring the distinctive cyberpunk aesthetic.

## Features

### Code Editor
- Monaco Editor (VSCode engine)
- Aria syntax highlighting
- Intelligent code completion
- Error underlining
- Cyberpunk theme
- Vim/Emacs keybindings (optional)

### Execution Environment
- WASM-based Aria compiler
- Sandboxed execution
- Real-time output
- Memory usage visualization
- Performance profiling

### Sharing & Collaboration
- Generate shareable links
- Embed snippets in blogs
- Fork existing snippets
- Comment and discuss
- Version history

## Tech Stack

### Frontend
```
- SvelteKit / Next.js
- Monaco Editor
- TailwindCSS
- Three.js (background effects)
- Web Workers (execution)
```

### WASM Integration
```
- Rust → WASM compilation
- wasm-bindgen for JS interop
- Web Workers for isolation
- SharedArrayBuffer for performance
```

### Backend API
```
- Snippet storage: PostgreSQL
- Caching: Redis
- CDN: Cloudflare
- Authentication: Auth0/Supabase
```

## Architecture

```
┌─────────────────────────────────────┐
│          Web Interface              │
│  (SvelteKit + Monaco Editor)        │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│       WASM Compiler Module          │
│  (aria-compiler compiled to WASM)   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│        Execution Sandbox            │
│     (Web Worker + WASM)             │
└─────────────────────────────────────┘
```

## Development

### Prerequisites
```bash
# Install dependencies
node >= 18
rust >= 1.70
wasm-pack
```

### Setup
```bash
cd aria-web/playground

# Install npm dependencies
npm install

# Build WASM module
cd ../../crates/aria-compiler
wasm-pack build --target web

# Copy WASM to playground
cp pkg/* ../../aria-web/playground/src/lib/wasm/

# Run dev server
cd ../../aria-web/playground
npm run dev
```

### File Structure
```
playground/
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── Editor.svelte
│   │   │   ├── Output.svelte
│   │   │   ├── Toolbar.svelte
│   │   │   └── ThemeToggle.svelte
│   │   ├── wasm/
│   │   │   ├── compiler.js
│   │   │   └── aria_bg.wasm
│   │   └── utils/
│   │       ├── syntax.ts
│   │       └── formatter.ts
│   ├── routes/
│   │   ├── +page.svelte
│   │   ├── share/[id]/+page.svelte
│   │   └── embed/[id]/+page.svelte
│   └── app.html
├── static/
│   ├── themes/
│   │   └── cyberpunk.json
│   └── examples/
│       ├── hello.aria
│       ├── fibonacci.aria
│       └── contracts.aria
└── package.json
```

## Features in Detail

### 1. Syntax Highlighting

Custom Aria syntax for Monaco:
```typescript
// syntax.ts
export const ariaLanguage = {
  keywords: [
    'fn', 'let', 'mut', 'if', 'else', 'match',
    'for', 'while', 'loop', 'return', 'break',
    'continue', 'struct', 'enum', 'trait', 'impl',
    'requires', 'ensures', 'invariant', 'spawn'
  ],
  operators: [
    '=', '>', '<', '!', '?', ':',
    '==', '<=', '>=', '!=', '&&', '||',
    '+', '-', '*', '/', '%', '**'
  ],
  symbols: /[=><!~?:&|+\-*\/\^%]+/,
  // ... more configuration
};
```

### 2. WASM Compiler Interface

```typescript
// compiler.ts
import init, { compile, format } from './wasm/compiler';

export class AriaCompiler {
  async init() {
    await init();
  }

  async compile(source: string): Promise<CompileResult> {
    try {
      const result = compile(source);
      return {
        success: true,
        output: result.output,
        errors: [],
        warnings: result.warnings
      };
    } catch (error) {
      return {
        success: false,
        output: '',
        errors: [error.message],
        warnings: []
      };
    }
  }

  format(source: string): string {
    return format(source);
  }
}
```

### 3. Execution Sandbox

```typescript
// executor.ts
export class SandboxExecutor {
  private worker: Worker;

  constructor() {
    this.worker = new Worker('/workers/executor.js');
  }

  async execute(code: string): Promise<ExecutionResult> {
    return new Promise((resolve) => {
      this.worker.postMessage({ type: 'execute', code });

      this.worker.onmessage = (e) => {
        if (e.data.type === 'result') {
          resolve({
            stdout: e.data.stdout,
            stderr: e.data.stderr,
            exitCode: e.data.exitCode,
            executionTime: e.data.time
          });
        }
      };

      // Timeout after 5 seconds
      setTimeout(() => {
        this.worker.terminate();
        this.worker = new Worker('/workers/executor.js');
        resolve({
          stdout: '',
          stderr: 'Execution timeout',
          exitCode: -1,
          executionTime: 5000
        });
      }, 5000);
    });
  }
}
```

### 4. Share Functionality

```typescript
// share.ts
export async function shareSnippet(code: string): Promise<string> {
  const response = await fetch('/api/snippets', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      code,
      language: 'aria',
      theme: 'cyberpunk'
    })
  });

  const { id } = await response.json();
  return `${window.location.origin}/share/${id}`;
}
```

## Examples

Sample programs to include:

### Hello World
```aria
fn main()
  print("Hello, Aria World!")
end
```

### Fibonacci with Contracts
```aria
fn fibonacci(n: Int) -> Int
  requires n >= 0 : "n must be non-negative"
  ensures result >= 0

  if n <= 1
    return n
  end

  return fibonacci(n - 1) + fibonacci(n - 2)
end
```

### Concurrent Hello
```aria
fn main()
  spawn { print("From agent 1") }
  spawn { print("From agent 2") }

  sleep(100)
end
```

## Deployment

### Build for Production
```bash
npm run build
```

### Deploy to Vercel
```bash
vercel deploy --prod
```

### Deploy to Cloudflare Pages
```bash
npm run build
wrangler pages publish dist
```

## Performance Optimization

- Lazy load WASM module
- Code splitting
- Service worker caching
- CDN for static assets
- Debounced compilation
- Web Worker for heavy computation

## Accessibility

- Keyboard navigation
- Screen reader support
- High contrast mode
- Reduced motion support
- Focus indicators

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 15+
- Mobile browsers (with limitations)

## Contributing

See [CONTRIBUTING.md](../../community/CONTRIBUTING.md)

---

*"Code in the cyberpunk playground, where the future compiles today."*
