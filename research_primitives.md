# Agent-Native Language Primitives Research for aria-lang

This document explores native language constructs designed for AI agents, focusing on safety, transparency, and human-AI cooperation.

## 1. 'think' Blocks: Native Reasoning
Reasoning is often handled as "hidden" text in prompt engineering. In `aria-lang`, it becomes a first-class citizen.

### Syntax
```aria
think {
    "Analyzing the user request..."
    "Plan: 1. Read files, 2. Apply patch."
}
```
### Technical Purpose
- **Observability**: The runtime can stream `think` blocks to a monitoring UI without executing them.
- **Traceability**: Reasoning is coupled with action in the AST (Abstract Syntax Tree).
- **Optimization**: Compilers can use reasoning hints for predictive resource allocation.

## 2. Tool-Calling Syntax
Tool use should be typed and scoped.

### Syntax
```aria
use tool filesystem

action save_report(data: Json) {
    filesystem.write("report.json", data)
}
```
### Technical Purpose
- **Capability Scoping**: Tools are explicitly imported/granted to an agent scope.
- **Schema Validation**: Tool signatures are checked at parse-time.

## 3. Human-in-the-Loop (HITL) Validation Gates
Safety is hard-coded into the control flow.

### Syntax
```aria
gate "Dangerous operation: Delete database?" {
    db.drop_all()
}
```
### Technical Purpose
- **Blocking Execution**: The runtime pauses and awaits human signature/approval.
- **Contextual Awareness**: The `gate` block captures the state for the human to review.

## 5. Agentic Error Handling and Recovery
In standard languages, errors are exceptions. In `aria-lang`, errors can trigger a "recovery thought" cycle.

### Syntax
```aria
try {
    filesystem.delete("/protected/file")
} recover (err) {
    think {
        "Deletion failed: ${err.message}. Checking permissions."
    }
    // Attempt alternative or escalate to human
    gate "Permission denied. Escalate to root?" {
        sudo.filesystem.delete("/protected/file")
    }
}
```

## 6. Safety by Design: The 'Cooperation' Principle
- **Explicit Delegation**: The `delegate` keyword makes it clear when control is passed from human/main script to an autonomous agent.
- **Intent vs Action**: `propose` primitives allow agents to "dry run" actions for human review without side effects.
- **Thought Integrity**: The runtime ensures that `think` blocks cannot be bypassed or hidden during execution, ensuring a "Reasoning-Action-Result" (RAR) loop that is fully auditable.

---

## Code Examples

### Example 1: Autonomous File Management with Safety Gates
```aria
// aria-lang/examples/safe_file_agent.aria

import "std/fs" as fs
import "std/ui" as ui

agent SecurityBot {
    capability fs.write
    capability fs.read

    fn patch_security_vulnerability(file_path: string) {
        think {
            "Detected a potential XSS in ${file_path}."
            "Applying the standard sanitization wrapper."
        }

        let content = fs.read(file_path)
        let patched = content.replace("<script>", "&lt;script&gt;")

        // HITL Gate: Requires human approval for file writes
        gate "Approve security patch for ${file_path}?" {
            fs.write(file_path, patched)
            ui.notify("Patch applied successfully.")
        }
    }
}
```

### Example 2: Cooperative Multi-Agent Planning
```aria
// aria-lang/examples/cooperative_planning.aria

agent Architect {
    task design_system() {
        think { "Defining the core data models." }
        let spec = { ... }
        
        // Propose a change to another agent or human
        propose "System Specification" to Human {
            spec
        }
    }
}
```
