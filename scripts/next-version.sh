#!/usr/bin/env bash
# Calcula a próxima versão a partir dos commits (Conventional Commits) desde a última tag vX.Y.Z.
#   feat → minor · fix/perf → patch · "!" ou BREAKING CHANGE → major (minor enquanto 0.x)
#   outros (docs, build, test, chore…) → sem versão nova
# Uso: scripts/next-version.sh [auto|patch|minor|major]
# Saída (stdout, formato do $GITHUB_OUTPUT): version=…, release=true|false, notes em notes.md
set -euo pipefail
force="${1:-auto}"

last=$(git tag --list 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | head -n1)
last=${last:-v0.0.0}
range="$last..HEAD"
git rev-parse -q --verify "refs/tags/$last" >/dev/null || range="HEAD"

subjects=$(git log --format='%s' "$range")
bodies=$(git log --format='%b' "$range")

bump=none
if grep -qE '^[a-z]+(\([^)]*\))?!:' <<<"$subjects" || grep -q 'BREAKING CHANGE' <<<"$bodies"; then
  bump=major
elif grep -qE '^feat(\([^)]*\))?:' <<<"$subjects"; then
  bump=minor
elif grep -qE '^(fix|perf)(\([^)]*\))?:' <<<"$subjects"; then
  bump=patch
fi
[[ "$force" != "auto" ]] && bump="$force"

IFS=. read -r major minor patch <<<"${last#v}"
case "$bump" in
  major) if (( major == 0 )); then minor=$((minor + 1)); patch=0; else major=$((major + 1)); minor=0; patch=0; fi ;;
  minor) minor=$((minor + 1)); patch=0 ;;
  patch) patch=$((patch + 1)) ;;
esac
version="$major.$minor.$patch"

# notas da versão: novidades e correções (sem o prefixo técnico)
{
  feats=$(grep -E '^feat(\([^)]*\))?!?:' <<<"$subjects" | sed -E 's/^feat(\(([^)]*)\))?!?: */- **\2** /; s/- \*\*\*\* /- /' || true)
  fixes=$(grep -E '^(fix|perf)(\([^)]*\))?!?:' <<<"$subjects" | sed -E 's/^(fix|perf)(\(([^)]*)\))?!?: */- **\3** /; s/- \*\*\*\* /- /' || true)
  [[ -n "$feats" ]] && printf '## Novidades\n\n%s\n\n' "$feats"
  [[ -n "$fixes" ]] && printf '## Correções\n\n%s\n\n' "$fixes"
  printf '## Downloads\n\n'
  printf -- '- Windows: `NucleoERP-Windows-Instalador.exe` (ou `.msi`)\n'
  printf -- '- Linux: `NucleoERP-Linux.AppImage`, `.deb` ou `.rpm`\n\n'
  printf 'Alterações desde %s.\n' "$last"
} > notes.md

echo "previous=$last"
echo "version=$version"
if [[ "$bump" == "none" ]]; then echo "release=false"; else echo "release=true"; fi
