# Kiro CLI post-tool-use hook (v2: postToolUse; v3: PostToolUse).
. "$PSScriptRoot\..\lib\ai-memory-hook.ps1"
Invoke-AiMemoryHook -Event "post-tool-use" -Agent "kiro-cli"
exit 0
