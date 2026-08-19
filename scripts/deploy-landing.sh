#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SITE_DIR="$REPO_ROOT/landing/site"
ENV_FILE="$REPO_ROOT/scripts/deploy-landing.env"

DRY_RUN=0
PRUNE=0

usage() {
  cat <<'USAGE'
Deploy the landing page to the VPS that serves claudemux.com.

    scripts/deploy-landing.sh [--dry-run] [--prune] [--yes]

Rebuilds landing/site/index.html from its three sources, then rsyncs
landing/site/ to the web root over SSH.

  --dry-run   Show exactly what would transfer; change nothing.
  --prune     Also delete remote files that no longer exist locally.
              Off by default, because deleting is the only way this
              script can destroy something.
  --yes       Skip the confirmation prompt (for CI).

The installer directory dl/ is excluded unconditionally, including under
--prune. It is owned by .github/workflows/release.yml, holds the .dmg and
.exe the landing links to, and is not reproducible from this repo.

Configuration, from scripts/deploy-landing.env or the environment:

  LANDING_HOST    required   ssh host, e.g. claudemux.com
  LANDING_USER    required   ssh user
  LANDING_PATH    required   web root, e.g. /var/www/claudemux
                             (NOT .../dl — that is the installer dir)
  LANDING_PORT    optional   ssh port, default 22
  LANDING_SSH_KEY optional   private key path, default ssh's own resolution
  LANDING_URL     optional   public URL to verify, default https://claudemux.com/

scripts/deploy-landing.env is gitignored. Copy the block above into it as
KEY=value lines.
USAGE
}

die() { printf 'error: %s\n' "$1" >&2; exit 1; }

ASSUME_YES=0
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=1 ;;
    --prune) PRUNE=1 ;;
    --yes|-y) ASSUME_YES=1 ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (try --help)" ;;
  esac
  shift
done

if [ -f "$ENV_FILE" ]; then
  set -a
  . "$ENV_FILE"
  set +a
fi

: "${LANDING_PORT:=22}"
: "${LANDING_URL:=https://claudemux.com/}"

[ -n "${LANDING_HOST:-}" ] || die "LANDING_HOST is not set (see --help)"
[ -n "${LANDING_USER:-}" ] || die "LANDING_USER is not set (see --help)"
[ -n "${LANDING_PATH:-}" ] || die "LANDING_PATH is not set (see --help)"

case "$LANDING_PATH" in
  /|/root|/home|/var|/var/www|/etc|/usr|"")
    die "LANDING_PATH='$LANDING_PATH' is too broad to deploy into safely" ;;
  */dl|*/dl/)
    die "LANDING_PATH='$LANDING_PATH' points at the installer directory; use the web root above it" ;;
  /*) ;;
  *) die "LANDING_PATH must be an absolute path, got '$LANDING_PATH'" ;;
esac

command -v rsync >/dev/null || die "rsync not found"

printf '==> Rebuilding landing/site/index.html\n'
( cd "$REPO_ROOT" && npm run --silent landing:build )

[ -s "$SITE_DIR/index.html" ] || die "$SITE_DIR/index.html is missing or empty after the build"

if [ -f "$REPO_ROOT/landing/_og.svg" ] && [ -f "$SITE_DIR/og.png" ] \
   && [ "$REPO_ROOT/landing/_og.svg" -nt "$SITE_DIR/og.png" ]; then
  printf 'warning: landing/_og.svg is newer than site/og.png.\n'
  printf '         og.png is a manual 1200x630 export; re-export it or link\n'
  printf '         previews will keep showing the old image.\n'
fi

SSH_CMD="ssh -p $LANDING_PORT"
[ -n "${LANDING_SSH_KEY:-}" ] && SSH_CMD="$SSH_CMD -i $LANDING_SSH_KEY"

RSYNC_FLAGS=(-rlptz --itemize-changes --exclude 'dl' --exclude 'dl/**' --exclude '.DS_Store')
[ "$PRUNE" -eq 1 ] && RSYNC_FLAGS+=(--del)
[ "$DRY_RUN" -eq 1 ] && RSYNC_FLAGS+=(-n)

DEST="$LANDING_USER@$LANDING_HOST:$LANDING_PATH/"

printf '\n==> Plan\n'
printf '    from    %s/\n' "$SITE_DIR"
printf '    to      %s\n' "$DEST"
printf '    port    %s\n' "$LANDING_PORT"
printf '    prune   %s\n' "$([ "$PRUNE" -eq 1 ] && echo 'yes — remote-only files will be DELETED' || echo 'no')"
printf '    dl/     excluded (installers are never touched)\n'
[ "$DRY_RUN" -eq 1 ] && printf '    mode    DRY RUN — nothing will change\n'
printf '\n'

if [ "$DRY_RUN" -eq 0 ] && [ "$ASSUME_YES" -eq 0 ]; then
  printf 'Deploy to %s? [y/N] ' "$LANDING_HOST"
  read -r reply
  case "$reply" in
    y|Y|yes|YES) ;;
    *) printf 'Aborted.\n'; exit 1 ;;
  esac
fi

printf '==> Syncing\n'
rsync "${RSYNC_FLAGS[@]}" -e "$SSH_CMD" "$SITE_DIR/" "$DEST"

if [ "$DRY_RUN" -eq 1 ]; then
  printf '\nDry run complete. Nothing changed.\n'
  exit 0
fi

printf '\n==> Verifying %s\n' "$LANDING_URL"
LIVE="$(mktemp)"
trap 'rm -f "$LIVE"' EXIT

if ! curl -fsS --max-time 30 "$LANDING_URL" -o "$LIVE"; then
  die "deployed, but $LANDING_URL did not respond — check the web server"
fi

if diff -q "$LIVE" "$SITE_DIR/index.html" >/dev/null; then
  printf '    live page matches the local build\n'
  printf '\nDone.\n'
else
  printf 'warning: live page differs from the local build.\n'
  printf '         Usually a CDN or Caddy cache; re-check in a minute.\n'
  printf '         Local %s bytes, live %s bytes.\n' \
    "$(wc -c < "$SITE_DIR/index.html" | tr -d ' ')" "$(wc -c < "$LIVE" | tr -d ' ')"
  exit 1
fi
