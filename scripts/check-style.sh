#!/usr/bin/env bash
# SPDX-License-Identifier: copyleft-next-0.3.1
# Style and commit-message checks. Run from the checkout:
#   scripts/check-style.sh
# Needs git, awk, sed, grep and python3. Hand-authored files only: the
# license texts under LICENSES/ stay byte-for-byte and the images are
# binary, so both are exempt.
set -o errexit -o nounset -o pipefail

# A patch keeps the whitespace of the lines it quotes.
scope=(-- . ':!LICENSES' ':!*.png' ':!*.ico' ':!*.patch')
status=0
here="$(dirname "${BASH_SOURCE[0]}")"

# Commit message trailers, as the kernel's checkpatch.pl checks them: a
# Signed-off-by is present and the last one is the committer's, an Assisted-by
# (if any) comes before one, and every Co-developed-by is immediately followed
# by the same person's Signed-off-by. Only the trailer block counts, read as
# `git interpret-trailers` reads it (without its patch divider, so a quoted
# hunk header in the body hides nothing), so a body line that quotes a tag is
# prose; tags match case-insensitively, as checkpatch matches them. A
# Generated-by is refused: Assisted-by replaced it.
check_trailers() {
		local where="$1" msg="$2" committer="$3" trailers last
		trailers="$(printf '%s\n' "$msg" | git interpret-trailers --parse --no-divider)"
		if ! printf '%s\n' "$trailers" | awk '
			tolower($0) ~ /^assisted-by:/ { assisted = 1 }
			tolower($0) ~ /^signed-off-by:/ && assisted { assisted = 0 }
			END { exit assisted }'; then
				echo "error: $where: 'Assisted-by:' must come before a 'Signed-off-by:'"
				status=1
		fi
		if ! printf '%s\n' "$trailers" | where="$where" committer="$committer" awk '
			pending {
				if (tolower($0) != ("signed-off-by: " tolower(person))) {
					print "error: " ENVIRON["where"] ": \"Co-developed-by: " person "\" must be immediately followed by \"Signed-off-by: " person "\""
					bad = 1
				}
				pending = 0
			}
			tolower($0) ~ /^co-developed-by:/ {
				person = $0
				sub(/^[^:]*:[[:space:]]*/, "", person)
				if (person == "") {
					print "error: " ENVIRON["where"] ": empty Co-developed-by"
					bad = 1
					next
				}
				if (tolower(person) == tolower(ENVIRON["committer"])) {
					print "error: " ENVIRON["where"] ": Co-developed-by names the committer, the primary author"
					bad = 1
				}
				pending = 1
			}
			END {
				if (pending) {
					print "error: " ENVIRON["where"] ": \"Co-developed-by: " person "\" has no Signed-off-by after it"
					bad = 1
				}
				exit bad
			}'; then
				status=1
		fi
		last="$(printf '%s\n' "$trailers" | grep --ignore-case '^Signed-off-by:' \
				| tail --lines=1 | sed 's/^[^:]*:[[:space:]]*//' || true)"
		if printf '%s\n' "$trailers" | grep --quiet --ignore-case '^Signed-off-by:[[:space:]]*$'; then
				echo "error: $where: empty Signed-off-by"
				status=1
		fi
		if printf '%s\n' "$trailers" | grep --quiet --ignore-case '^Generated-by:'; then
				echo "error: $where: 'Generated-by:' is replaced by 'Assisted-by: LLM'"
				status=1
		fi
		if [ -z "$last" ]; then
				echo "error: $where: missing Signed-off-by trailer"
				status=1
		elif [ "${last,,}" != "${committer,,}" ]; then
				echo "error: $where: the last Signed-off-by must be the committer's ($committer), not: $last"
				status=1
		fi
}

# `--commit-msg FILE` checks the trailers and the prose of the message in FILE
# and nothing else: the commit-msg hook's shape (scripts/commit-msg), so a bad
# message is refused before the commit exists rather than reported from HEAD
# afterwards. A verbose commit template carries the diff below git's scissors
# line, which stripspace keeps, so the file is cut there first, as git itself
# cuts it.
if [ "${1-}" = "--commit-msg" ]; then
		[ $# -eq 2 ] && [ -r "$2" ] || { echo "usage: $0 [--commit-msg FILE]" >&2; exit 2; }
		committer="$(git var GIT_COMMITTER_IDENT | sed 's/ [0-9]* [-+][0-9]*$//')"
		msg="$(sed '/^[#;] -* >8 -*$/,$d' "$2" | git stripspace --strip-comments)"
		check_trailers "commit message" "$msg" "$committer"
		printf '%s\n' "$msg" | python3 "$here/check-prose.py" --message - || status=1
		exit "$status"
fi

# 1. Trailing whitespace (spaces or tabs) and carriage returns.
if git grep --line-number --perl-regexp '[ \t]+$|\r' "${scope[@]}" >/dev/null 2>&1; then
		echo "error: trailing whitespace or a carriage return:"
		git grep --line-number --perl-regexp '[ \t]+$|\r' "${scope[@]}"
		status=1
fi

# 2. Missing newline at end of file.
while IFS= read -r f; do
		[ -s "$f" ] || continue
		if [ -n "$(tail --bytes=1 "$f")" ]; then
				echo "error: no newline at end of file: $f"
				status=1
		fi
done < <(git ls-files "${scope[@]}")

# 3. HEAD's commit message trailers (check_trailers above) and prose: its
#    message and the docs and comments it adds (scripts/check-prose.py,
#    notes/writing.md).
msg="$(git log --max-count=1 --format=%B 2>/dev/null || true)"
if [ -n "$msg" ]; then
		check_trailers HEAD "$msg" "$(git log --max-count=1 --format='%cn <%ce>')"
		python3 "$here/check-prose.py" --commit HEAD || status=1
fi

# 4. Every source file declares an SPDX license within its first two lines,
#    so a shebang may come first. scdoc pages cannot: their first line is
#    the preamble, and a comment before it is a parse error.
while IFS= read -r f; do
	if ! head --lines=2 "$f" | grep --quiet 'SPDX-License-Identifier: '; then
		echo "error: $f: missing 'SPDX-License-Identifier:' in the first two lines"
		status=1
	fi
done < <(git ls-files "${scope[@]}" | grep --extended-regexp '\.(rs|nix|sh|py|qemu|conf|rst|css|yaml|jsonc|ini)$|^(scripts|steps/[^/]*)/[^./]*$|^tests/(verify|update-expected)$' || true)

[ "$status" -eq 0 ] && echo "style: OK"
exit "$status"
