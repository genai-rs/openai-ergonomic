#!/bin/bash

# Script to clean AI attribution from git history
# WARNING: This rewrites git history and requires force push

set -euo pipefail

echo "⚠️  WARNING: This will rewrite git history to remove AI attribution"
echo "This is a destructive operation that requires force push"
read -p "Are you sure you want to continue? (yes/no): " confirmation

if [ "$confirmation" != "yes" ]; then
    echo "Aborted."
    exit 0
fi

echo "🔍 Creating backup branch..."
git branch backup-before-attribution-cleanup-$(date +%Y%m%d-%H%M%S)

echo "🧹 Cleaning commit messages..."

# Use git filter-branch to clean commit messages
git filter-branch -f --msg-filter '
    sed -e "/🤖 Generated with \[Claude Code\]/d" \
        -e "/🤖 Generated with/d" \
        -e "/Co-authored-by: Claude/d" \
        -e "/Co-Authored-By: Claude/d" \
        -e "/Generated with Claude/d" \
        -e "/Created with Claude/d" \
        -e "/AI-generated/d" \
        -e "/LLM-generated/d" |
    sed -e "/^[[:space:]]*$/d" |
    awk '\''NF'\'' |
    cat -s
' --tag-name-filter cat -- --all

echo "✅ Commit messages cleaned"

echo "🔍 Verifying no attribution remains..."
if git log --all --format="%B" | grep -iE "(Claude|Anthropic|🤖|Co-authored-by.*Claude|Generated with)" > /dev/null 2>&1; then
    echo "❌ Attribution still found! Manual intervention may be needed."
    git log --all --grep="Claude" --grep="🤖" --format="%h %s"
    exit 1
else
    echo "✅ No AI attribution found in commit history"
fi

echo ""
echo "🎉 Success! Git history has been cleaned."
echo ""
echo "⚠️  IMPORTANT NEXT STEPS:"
echo "1. Review the changes with: git log --oneline -20"
echo "2. Force push to remote with: git push --force-with-lease origin main"
echo "3. All team members will need to re-clone or reset their local repos"
echo ""
echo "Backup branch created: backup-before-attribution-cleanup-*"