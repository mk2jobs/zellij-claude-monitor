#!/usr/bin/env python3
"""Claude Code statusline bridge.

Reads session data from Claude Code stdin, saves to ~/.claude/statusline.json
for the Dashboard plugin to consume.
"""
import sys
import json
import os


def main():
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return

    path = os.path.expanduser("~/.claude/statusline.json")
    tmp = path + ".tmp"
    try:
        with open(tmp, "w") as f:
            json.dump(data, f)
        os.replace(tmp, path)
    except OSError:
        pass

    # Output for Claude Code's own status display
    model = data.get("model", {}).get("display_name", "")
    ctx = int(data.get("context_window", {}).get("used_percentage", 0))
    proj = os.path.basename(data.get("workspace", {}).get("project_dir", ""))
    print(f"{model} | {proj} | Ctx: {ctx}%")


if __name__ == "__main__":
    main()
