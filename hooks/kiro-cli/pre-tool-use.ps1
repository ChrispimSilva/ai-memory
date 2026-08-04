# Kiro CLI pre-tool-use hook (v2: preToolUse; v3: PreToolUse).
# Fail-open by contract: exit 0 unconditionally and print nothing —
# exit code 2 would block the tool call on both engines.
. "$PSScriptRoot\..\lib\ai-memory-hook.ps1"
Invoke-AiMemoryHook -Event "pre-tool-use" -Agent "kiro-cli"
exit 0
