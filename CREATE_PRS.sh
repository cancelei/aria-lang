#!/bin/bash

# Script to create PRs for Day 3-7 work

echo "Creating Day 3 Parser PR..."
gh pr create \
  --title "Day 3 (Part 1): Parser Foundation - Tools, Agents & Orchestration" \
  --body-file PR_DAY3_BODY.md \
  --base master \
  --head day3-parser-foundation \
  --label "contest-day-3" \
  --label "parser" \
  --label "enhancement"

echo ""
echo "Creating draft PRs for upcoming days..."

# Day 3 Part 2 - Evaluator
cat > /tmp/pr_day3_part2.md << 'EOF'
# Day 3 (Part 2): Evaluator - Runtime Physics

Implement the runtime execution engine:
- Tool registration & execution
- Agent spawning with scoped variables
- **Permission checking** (physics-based safety!)
- Delegation with task invocation

**Depends on**: Day 3 Part 1 (parser)
**Status**: Planning
**Estimated**: 4-6 hours

See `CONTINUATION_GUIDE.md` for implementation details.
EOF

gh pr create --draft \
  --title "Day 3 (Part 2): Evaluator - Runtime Physics & Permission System" \
  --body-file /tmp/pr_day3_part2.md \
  --base master \
  --head day3-parser-foundation \
  --label "contest-day-3" \
  --label "evaluator" \
  --label "enhancement" \
  || echo "Draft PR creation skipped (branch not ready)"

# Day 4 - Sandbox
cat > /tmp/pr_day4.md << 'EOF'
# Day 4: The Nervous System - Sandbox & Isolation

Implement sandboxed execution for risky operations:
- Docker/WASM sandboxing
- Resource limits (CPU, memory, timeout)
- Process isolation
- Audit logging

**Depends on**: Day 3 Part 2 (evaluator)
**Status**: Planning
**Vision**: From VISION.md - "The Membrane"
EOF

gh issue create \
  --title "Day 4: Implement Sandbox Membrane for Tool Isolation" \
  --body-file /tmp/pr_day4.md \
  --label "contest-day-4" \
  --label "security" \
  --label "enhancement" \
  || echo "Issue creation skipped"

# Day 5 - Immune System
cat > /tmp/pr_day5.md << 'EOF'
# Day 5: The Immune System - Safety & Resource Management

Comprehensive safety and resource management:
- Permission model refinement
- Rate limiting for tools
- Error recovery mechanisms
- Resource quota enforcement

**Depends on**: Day 4 (sandbox)
**Status**: Planning
**Vision**: From VISION.md - "The Immune System"
EOF

gh issue create \
  --title "Day 5: Implement Immune System - Safety & Resource Management" \
  --body-file /tmp/pr_day5.md \
  --label "contest-day-5" \
  --label "security" \
  --label "enhancement" \
  || echo "Issue creation skipped"

# Day 6 - Voice
cat > /tmp/pr_day6.md << 'EOF'
# Day 6: The Voice - Standard Library & Built-in Tools

Build the standard library:
- File I/O tools
- HTTP/network tools
- Common agent primitives
- Integration with Moltbook

**Depends on**: Day 5 (immune system)
**Status**: Planning
**Vision**: From VISION.md - "The Voice"
EOF

gh issue create \
  --title "Day 6: Implement Standard Library - The Voice" \
  --body-file /tmp/pr_day6.md \
  --label "contest-day-6" \
  --label "stdlib" \
  --label "enhancement" \
  || echo "Issue creation skipped"

# Day 7 - Launch
cat > /tmp/pr_day7.md << 'EOF'
# Day 7: The Organism - v1.0 Launch

Final integration and launch:
- Comprehensive integration tests
- Documentation polish
- Example programs
- Performance testing
- v1.0 release preparation

**Depends on**: Day 6 (stdlib)
**Status**: Planning
**Vision**: From VISION.md - "The Organism"
**Goal**: Contest submission ready!
EOF

gh issue create \
  --title "Day 7: Final Integration & v1.0 Launch - The Organism" \
  --body-file /tmp/pr_day7.md \
  --label "contest-day-7" \
  --label "release" \
  --label "enhancement" \
  || echo "Issue creation skipped"

echo ""
echo "✅ PR and issues created!"
echo ""
echo "View with: gh pr list && gh issue list"
