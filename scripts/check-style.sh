#!/usr/bin/env bash
# SPDX-License-Identifier: copyleft-next-0.3.1
set -o errexit -o nounset -o pipefail

# LICENSES/ stays byte-for-byte, images are binary, and a patch keeps the
# whitespace of the lines it quotes.
scope=(-- . ':!LICENSES' ':!*.png' ':!*.ico' ':!*.patch')
status=0
here="$(dirname "${BASH_SOURCE[0]}")"

# Trailers as checkpatch.pl checks them, case-insensitively. Only the
# trailer block counts, parsed without the patch divider so that a hunk
# header quoted in the body hides nothing.
check_trailers() {
		local where="$1" msg="$2" committer="$3" trailers tag last
		trailers="$(printf '%s\n' "$msg" | git interpret-trailers --parse --no-divider)"
		tag="$(printf '%s\n' "$trailers" | grep --ignore-case --only-matching --max-count=1 \
				--extended-regexp '^(assisted-by|generated-by|co-authored-by):' || true)"
		if [ -n "$tag" ]; then
				echo "error: $where: '$tag' is refused: whoever signs off answers for the change"
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
		if [ -z "$last" ]; then
				echo "error: $where: missing Signed-off-by trailer"
				status=1
		elif [ "${last,,}" != "${committer,,}" ]; then
				echo "error: $where: the last Signed-off-by must be the committer's ($committer), not: $last"
				status=1
		fi
}

# A verbose commit template carries the diff below git's scissors line,
# which stripspace keeps, so cut there first as git does.
if [ "${1-}" = "--commit-msg" ]; then
		[ $# -eq 2 ] && [ -r "$2" ] || { echo "usage: $0 [--commit-msg FILE]" >&2; exit 2; }
		committer="$(git var GIT_COMMITTER_IDENT | sed 's/ [0-9]* [-+][0-9]*$//')"
		msg="$(sed '/^[#;] -* >8 -*$/,$d' "$2" | git stripspace --strip-comments)"
		check_trailers "commit message" "$msg" "$committer"
		printf '%s\n' "$msg" | python3 "$here/check-prose.py" --message - || status=1
		exit "$status"
fi

if git grep --line-number --perl-regexp '[ \t]+$|\r' "${scope[@]}" >/dev/null 2>&1; then
		echo "error: trailing whitespace or a carriage return:"
		git grep --line-number --perl-regexp '[ \t]+$|\r' "${scope[@]}"
		status=1
fi

while IFS= read -r f; do
		[ -s "$f" ] || continue
		if [ -n "$(tail --bytes=1 "$f")" ]; then
				echo "error: no newline at end of file: $f"
				status=1
		fi
done < <(git ls-files "${scope[@]}")

msg="$(git log --max-count=1 --format=%B 2>/dev/null || true)"
if [ -n "$msg" ]; then
		check_trailers HEAD "$msg" "$(git log --max-count=1 --format='%cn <%ce>')"
		python3 "$here/check-prose.py" --commit HEAD || status=1
fi

# Two lines, so a shebang may come first. Not .scd: a comment before
# scdoc's preamble line is a parse error.
while IFS= read -r f; do
	if ! head --lines=2 "$f" | grep --quiet 'SPDX-License-Identifier: '; then
		echo "error: $f: missing 'SPDX-License-Identifier:' in the first two lines"
		status=1
	fi
done < <(git ls-files "${scope[@]}" | grep --extended-regexp '\.(rs|nix|sh|py|qemu|conf|rst|css|yaml|jsonc|ini)$|^(scripts|steps/[^/]*)/[^./]*$|^tests/(verify|update-expected)$' || true)

[ "$status" -eq 0 ] && echo "style: OK"
exit "$status"
