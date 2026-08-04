-- Per-client MCP tool-call counters, bucketed by UTC day.
--
-- Hook-driven agents surface in `sessions.agent_kind` (and the
-- `/admin/sessions/by-agent` aggregate), but MCP-only clients — VS Code
-- Copilot, Claude Desktop, ad-hoc scripts — never fire a lifecycle hook,
-- so they were invisible to "where is this server's memory traffic
-- coming from". This table records what those clients actually do:
-- tool calls, split into reads and writes.
--
-- `client` is the sanitized MCP `clientInfo.name` from the initialize
-- handshake when the HTTP transport runs stateful, else the
-- `X-Memory-Actor-Agent` overlay when an ingress proxy asserts one,
-- else the literal 'unknown'. Free text on purpose: MCP client names
-- are an open set, and forcing them through the `AgentKind` CHECK
-- would misfile every client the enum has not met yet.
--
-- One row per (client, day) keeps the table O(clients × days) — it
-- grows with the calendar, not with traffic.
CREATE TABLE client_activity (
    client  TEXT NOT NULL,
    day     INTEGER NOT NULL,             -- UTC days since the epoch
    reads   INTEGER NOT NULL DEFAULT 0 CHECK (reads >= 0),
    writes  INTEGER NOT NULL DEFAULT 0 CHECK (writes >= 0),
    PRIMARY KEY (client, day)
) WITHOUT ROWID;
