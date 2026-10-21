#!/usr/bin/env python3
"""Probe every Git source in a Campaign revision's locked closure without credentials.

Walks every flake.lock node reachable from the root (resolving `follows`) and every
`git+` source in the pinned Cairn and Octet Cargo.lock files, then runs `git ls-remote`
on each exact locked URL with Git configuration, prompts, SSH configuration, identities,
and agent disabled.

Usage: closure-probe.py <campaign-repo> <revision> <cairn-clone> <octet-clone> <tools-store-path>
The Cairn and Octet clones are read locally at their pinned revisions because those
repositories refuse anonymous access.
"""
import collections
import json
import os
import re
import subprocess
import sys
import tempfile

CAIRN_REVISION = "5d5aa7dfc02e6cfd076a18c54b09954d23e29f2b"
OCTET_REVISION = "890f424cd7b1b2015f39c0eb5747f1bd1e576028"


def show(repo, revision, path):
    return subprocess.run(
        ["git", "-C", repo, "show", f"{revision}:{path}"],
        capture_output=True, text=True, check=True,
    ).stdout


def main():
    repo, revision, cairn, octet, tools = sys.argv[1:6]
    lock = json.loads(show(repo, revision, "flake.lock"))
    nodes = lock["nodes"]

    def resolve(ref):
        if isinstance(ref, str):
            return ref
        node = lock["root"]
        for step in ref:
            node = resolve(nodes[node]["inputs"][step])
        return node

    seen, queue = set(), collections.deque([lock["root"]])
    while queue:
        node = queue.popleft()
        if node in seen:
            continue
        seen.add(node)
        queue.extend(resolve(ref) for ref in nodes[node].get("inputs", {}).values())

    targets = {}

    def add(url, rev, origin):
        targets.setdefault(url, {}).setdefault(rev, set()).add(origin)

    for node in seen - {lock["root"]}:
        locked = nodes[node].get("locked", {})
        if locked.get("type") == "github":
            add(f"https://github.com/{locked['owner']}/{locked['repo']}", locked["rev"], "flake.lock")
        elif locked.get("type") == "git":
            add(locked["url"], locked["rev"], "flake.lock")
    for name, clone, rev in (("Cairn", cairn, CAIRN_REVISION), ("Octet", octet, OCTET_REVISION)):
        for match in re.finditer(r'source = "git\+([^?"]+)\?rev=([0-9a-f]{40})', show(clone, rev, "Cargo.lock")):
            add(match.group(1), match.group(2), f"{name} Cargo.lock")

    home = tempfile.mkdtemp(prefix="probe-home.")
    certs = f"{tools}/etc/ssl/certs/ca-bundle.crt"
    env = {
        "HOME": home,
        "PATH": f"{tools}/bin",
        "SSL_CERT_FILE": certs,
        "GIT_SSL_CAINFO": certs,
        "GIT_TERMINAL_PROMPT": "0",
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": "/dev/null",
        "GIT_SSH_COMMAND": "ssh -F /dev/null -o IdentityFile=/dev/null -o IdentitiesOnly=yes"
        " -o IdentityAgent=none -o BatchMode=yes -o StrictHostKeyChecking=no"
        " -o UserKnownHostsFile=/dev/null",
    }
    for url in sorted(targets, key=str.lower):
        result = subprocess.run(["git", "ls-remote", url, "HEAD"], env=env, capture_output=True, text=True, timeout=90)
        status = "answered" if result.returncode == 0 else "refused"
        revisions = "; ".join(f"{rev[:12]} ({', '.join(sorted(origins))})" for rev, origins in sorted(targets[url].items()))
        print(f"{status:8} exit={result.returncode:<3} {url} @ {revisions}")
        if result.returncode:
            lines = [line for line in result.stderr.splitlines() if line.strip()]
            reason = next((line for line in lines if "Permission denied" in line or "could not read" in line), lines[-1] if lines else "")
            print(f"         -> {reason.strip()}")


if __name__ == "__main__":
    main()
