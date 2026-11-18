#!/usr/bin/env python3
"""Fail if a commit subject tags a `[slice]` that docs/state-slices.md does not register,
or if AGENTS.md re-grows per-phase authorization prose."""
import re, subprocess, sys

registry = open("docs/state-slices.md").read()
known = set(re.findall(r"^## `([a-z0-9-]+)`", registry, re.M))
bad = []
for line in subprocess.check_output(["git", "log", "--format=%h %s"], text=True).splitlines():
    sha, _, subject = line.partition(" ")
    for tag in re.findall(r"\[([a-z0-9-]+-v\d+)\]\s*$", subject):
        if tag not in known:
            bad.append(f"{sha}: slice [{tag}] is not in docs/state-slices.md")
if re.search(r"authorized under named state slice", open("AGENTS.md").read()):
    bad.append("AGENTS.md: move 'authorized under named state slice' paragraphs to docs/state-slices.md")
if bad:
    print("\n".join(bad)); sys.exit(1)
print("slices ok")
