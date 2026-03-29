# Worktree Deletion Warnings

## Intent

Before removing a worktree on exit, check for uncommitted changes and unpushed commits. If either are present, warn the user and require confirmation before proceeding with removal.
