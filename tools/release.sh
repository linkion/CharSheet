#!/usr/bin/env bash
# Release helper, driven by the Makefile. Runs without prompts so Claude Code
# (or anyone) can use it from a script.
#
#   tools/release.sh changes            commits since the last release tag
#   tools/release.sh open VERSION [NOTES_FILE]
#       bumps the version on a release/vVERSION branch and opens a PR. Merging
#       it makes .github/workflows/release.yml tag the merge commit and publish
#       a GitHub release: NOTES_FILE's summary, then a generated changelog.
#   tools/release.sh notes VERSION NOTES_FILE
#       replaces the summary on the open release PR, or on the published
#       release once it is out. The changelog is left alone.
#
# The summary sits between the two marker comments below, in the PR body and
# in the release body alike.
set -euo pipefail

START='<!-- release-notes:start -->'
END='<!-- release-notes:end -->'
cd "$(git rev-parse --show-toplevel)"

die() { echo "release: $*" >&2; exit 1; }

normalize_version() {
    local v="${1#v}"
    [[ -n $v ]] || die "VERSION is required, e.g. make release VERSION=0.2.0"
    [[ $v =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] \
        || die "'$1' is not a semantic version like 0.2.0"
    echo "$v"
}

read_notes() {
    [[ -z ${1:-} ]] && return 0
    [[ -f $1 ]] || die "notes file '$1' does not exist"
    cat "$1"
}

# Prints stdin with the text between the markers replaced by file $1.
replace_notes() {
    awk -v f="$1" -v s="$START" -v e="$END" '
        $0 == s { print; while ((getline l < f) > 0) print l; skip = 1; next }
        $0 == e { skip = 0 }
        !skip'
}

last_tag() { git describe --tags --abbrev=0 --match 'v[0-9]*' "${1:-HEAD}" 2>/dev/null || true; }

changes() {
    local prev range
    prev="$(last_tag "${1:-HEAD}")"
    range="${prev:+$prev..}${1:-HEAD}"
    git log --no-merges --invert-grep --grep='^Release v[0-9]' \
        --pretty='- %s (%h)' "$range"
}

cmd_changes() {
    git fetch --quiet --tags origin || true
    local prev; prev="$(last_tag)"
    echo "Commits since ${prev:-the first commit}:"
    changes
}

cmd_open() {
    local version tag branch notes current today
    version="$(normalize_version "${1:-}")"
    tag="v$version"
    branch="release/$tag"
    notes="$(read_notes "${2:-}")"

    command -v gh >/dev/null || die "the GitHub CLI (gh) is required"
    gh auth status >/dev/null 2>&1 || die "gh is not logged in; run: gh auth login"
    [[ $(git branch --show-current) == master ]] || die "check out master first"
    [[ -z $(git status --porcelain) ]] || die "the working tree has uncommitted changes"

    git fetch --quiet --tags origin master
    [[ $(git rev-parse HEAD) == $(git rev-parse origin/master) ]] \
        || die "master and origin/master differ; push or pull first"
    git rev-parse -q --verify "refs/tags/$tag" >/dev/null && die "tag $tag already exists"
    git ls-remote --exit-code --heads origin "$branch" >/dev/null \
        && die "branch $branch already exists on origin (is a release PR already open?)"

    current="$(sed -n '/^\[workspace\.package\]/,/^\[/ s/^version = "\(.*\)"/\1/p' Cargo.toml)"
    today="$(date +%F)"
    echo "Releasing $tag (Cargo.toml has $current)"

    git switch --quiet -c "$branch"
    trap 'git switch --quiet master 2>/dev/null || true' EXIT

    sed -i "/^\[workspace\.package\]/,/^\[/ s/^version = \".*\"/version = \"$version\"/" Cargo.toml
    cargo update --workspace --offline --quiet

    local meta=packaging/io.github.linkion.CharSheet.metainfo.xml
    if grep -q "<release version=\"$version\"" "$meta"; then
        sed -i "s|<release version=\"$version\" date=\"[^\"]*\"|<release version=\"$version\" date=\"$today\"|" "$meta"
    else
        sed -i "s|^\(\s*\)<releases>$|&\n\1  <release version=\"$version\" date=\"$today\"/>|" "$meta"
    fi

    git add Cargo.toml Cargo.lock "$meta"
    git diff --cached --quiet && die "nothing changed; is $version already released?"
    git commit --quiet -m "Release $tag"
    git push --quiet -u origin "$branch"

    local body; body="$(mktemp)"
    {
        echo "Merging this tags the merge commit \`$tag\` and publishes a GitHub release."
        echo
        echo "## Summary"
        echo
        echo "Shown at the top of the release. Change it with"
        echo "\`make release-notes VERSION=$version NOTES=file.md\`, or edit it here, before merging."
        echo
        echo "$START"
        [[ -n $notes ]] && printf '%s\n' "$notes"
        echo "$END"
        echo
        echo "## Changelog preview"
        echo
        echo "Generated again from git history when the release is created."
        echo
        changes
    } > "$body"
    gh pr create --base master --head "$branch" --title "Release $tag" --body-file "$body"
    rm -f "$body"
}

cmd_notes() {
    local version tag branch file state body
    version="$(normalize_version "${1:-}")"
    tag="v$version"
    branch="release/$tag"
    file="${2:-}"
    [[ -n $file ]] || die "NOTES is required, e.g. make release-notes VERSION=$version NOTES=notes.md"
    read_notes "$file" >/dev/null
    body="$(mktemp)"
    trap 'rm -f "$body"' EXIT

    if gh release view "$tag" >/dev/null 2>&1; then
        local old; old="$(gh release view "$tag" --json body -q .body | tr -d '\r')"
        if grep -qxF "$START" <<<"$old"; then
            replace_notes "$file" <<<"$old" > "$body"
        else  # a release made by hand: put the summary on top
            { echo "$START"; cat "$file"; echo "$END"; echo; printf '%s\n' "$old"; } > "$body"
        fi
        gh release edit "$tag" --notes-file "$body"
        echo "Updated the summary on release $tag"
        return
    fi

    state="$(gh pr view "$branch" --json state -q .state 2>/dev/null)" \
        || die "no release or release PR found for $tag"
    case $state in
        OPEN)
            gh pr view "$branch" --json body -q .body | tr -d '\r' | replace_notes "$file" > "$body"
            grep -qxF "$START" "$body" || die "the PR body lost its release-notes markers; restore them"
            gh pr edit "$branch" --body-file "$body"
            echo "Updated the summary on the $tag release PR"
            ;;
        MERGED) die "the PR is merged but release $tag is not out yet; wait for the workflow, then retry" ;;
        *) die "the $tag release PR is $state" ;;
    esac
}

case "${1:-}" in
    changes) shift; cmd_changes "$@" ;;
    open) shift; cmd_open "$@" ;;
    notes) shift; cmd_notes "$@" ;;
    *) die "usage: $0 changes | open VERSION [NOTES_FILE] | notes VERSION NOTES_FILE" ;;
esac
