# Kiro CLI user-prompt hook (v2: userPromptSubmit; v3: UserPromptSubmit).
# Capture only — the handoff is injected by session-start.ps1. Print
# nothing: exit-0 stdout would be added to the conversation context.
. "$PSScriptRoot\..\lib\ai-memory-hook.ps1"
Invoke-AiMemoryHook -Event "user-prompt" -Agent "kiro-cli"
exit 0
