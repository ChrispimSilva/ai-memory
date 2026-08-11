#!/bin/sh
# Swival lifecycle hook (issue #385).
#
# Swival invokes `lifecycle_command` as positional args —
#     <command> startup <base_dir>
#     <command> exit <base_dir>
# — with the base_dir as cwd and SWIVAL_* env vars (SWIVAL_HOOK_EVENT,
# SWIVAL_BASE_DIR, ...). There is NO stdin payload.
#
# Swival captures hook stdout but never injects it into the model context,
# so this script only records the event. It never fetches /handoff:
# accepting a handoff is destructive and Swival would discard the output.
# Recover the prior session's handoff via the MCP memory_handoff_accept tool.
_lib_dir="$(dirname "$0")"
[ -f "$_lib_dir/_lib.sh" ] || _lib_dir="$_lib_dir/.."
. "$_lib_dir/_lib.sh"

event="$1"
base_dir="$2"
[ -n "$event" ] || exit 0
[ -n "$base_dir" ] || exit 0

SERVER="${AI_MEMORY_HOOK_URL:-http://127.0.0.1:49374}"
QS=$(ai_memory_marker_qs "$base_dir")

case "$event" in
    startup)
        printf '{}' \
            | ai_memory_post_hook "$SERVER/hook?event=session-start&agent=swival${QS}" >/dev/null 2>&1 || true
        ;;
    exit)
        printf '{}' \
            | ai_memory_post_hook "$SERVER/hook?event=session-end&agent=swival${QS}" >/dev/null 2>&1 || true
        ;;
esac
printf '{}\n'
exit 0
