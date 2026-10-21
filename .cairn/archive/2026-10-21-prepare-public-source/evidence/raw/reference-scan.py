#!/usr/bin/env python3
"""Resolve README links by type and scan tracked Markdown at one revision.

Web links get an anonymous HTTP GET, relative links must exist in the revision's tree,
and Git clone routes shown as code get an anonymous `git ls-remote`. Every tracked
Markdown file is scanned for `](../` links that leave the repository, `/home/` text,
and links to hosts that refuse anonymous access. Hits are reported by file and line only.

Usage: reference-scan.py <campaign-repo> <revision> <tools-store-path>
"""
import posixpath
import re
import subprocess
import sys
import tempfile

REFUSED_LINK = re.compile(
    r"github\.com/OnixResearch/(cairn|octet|animus|trellis|slotcar|valence|onix-artifact|mantle"
    r"|kiln|basalt|kamacite|ucan)(\.git)?([/)#`\s]|$)|git\.onix\.computer/z2scC9MCm3pxk9mX4FEidRKabQ5LN",
    re.IGNORECASE,
)


def git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)


def main():
    repo, revision, tools = sys.argv[1:4]
    certs = f"{tools}/etc/ssl/certs/ca-bundle.crt"
    env = {
        "HOME": tempfile.mkdtemp(prefix="scan-home."),
        "PATH": f"{tools}/bin",
        "SSL_CERT_FILE": certs,
        "GIT_SSL_CAINFO": certs,
        "GIT_TERMINAL_PROMPT": "0",
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": "/dev/null",
    }
    readme = git(repo, "show", f"{revision}:README.md").stdout
    for _, url in re.findall(r"\[([^\]]*)\]\(([^)\s]+)\)", readme):
        if url.startswith(("http://", "https://")):
            code = subprocess.run(
                ["curl", "-sS", "-L", "-o", "/dev/null", "-w", "%{http_code}", "--max-time", "60", url],
                env=env, capture_output=True, text=True,
            ).stdout.strip()
            print(f"web      {code or 'ERR'}  {url}")
        else:
            target = posixpath.normpath(url.split("#")[0])
            present = not target.startswith("..") and git(repo, "cat-file", "-e", f"{revision}:{target}").returncode == 0
            print(f"relative {'exists' if present else 'MISSING'}  {url}")
    for url in sorted(set(re.findall(r"`[^`]*?(https://[^`\s]+\.git)[^`]*`", readme))):
        answered = subprocess.run(["git", "ls-remote", url, "HEAD"], env=env, capture_output=True, timeout=90).returncode == 0
        print(f"clone    {'answered' if answered else 'REFUSED'}  {url}")

    markdown = [path for path in git(repo, "ls-tree", "-r", "--name-only", revision).stdout.splitlines() if path.endswith(".md")]
    print(f"--- tracked Markdown scan ({len(markdown)} files) ---")
    hits = 0
    for path in markdown:
        body = git(repo, "show", f"{revision}:{path}").stdout
        for number, line in enumerate(body.splitlines(), 1):
            for match in re.finditer(r"\]\((\.\./[^)\s]*)\)", line):
                target = posixpath.normpath(posixpath.join(posixpath.dirname(path), match.group(1).split("#")[0]))
                if target.startswith(".."):
                    print(f"escaping-link {path}:{number}")
                    hits += 1
            if "/home/" in line:
                print(f"home-text     {path}:{number}")
                hits += 1
            for match in re.finditer(r"\]\((https?://[^)\s]+)\)", line):
                if REFUSED_LINK.search(match.group(1)):
                    print(f"refused-link  {path}:{number}")
                    hits += 1
    print(f"{hits} hits")


if __name__ == "__main__":
    main()
