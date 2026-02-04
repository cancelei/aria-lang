#!/bin/bash
set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║     ARIA-LANG DAY 3 - PUSH AND CREATE PR                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

cd ~/Projects/aria-lang

# Check if branch exists on remote
echo "🔍 Checking if branch is already pushed..."
if git ls-remote --exit-code origin day3-parser-foundation &>/dev/null; then
    echo "✅ Branch already exists on remote!"
else
    echo "⏳ Pushing branch to remote..."
    git push -u origin day3-parser-foundation
    echo "✅ Branch pushed!"
fi

echo ""
echo "📝 Creating PR and issues..."
./CREATE_PRS.sh

echo ""
echo "✅ All done!"
echo ""
echo "View your PR:"
gh pr view --web day3-parser-foundation 2>/dev/null || \
gh pr list --head day3-parser-foundation

echo ""
echo "View issues:"
gh issue list --label contest

