# Python → Rust port map (inventory)

Source of truth: `tzervas/tg-agent-relay`. This file grows with each P22 phase.

| Python module | Rust crate | Status |
|---------------|------------|--------|
| `tg_agent_relay.send.paginate` | `tgar-telegram::pagination` | **P22c** |
| `tg_agent_relay.send.send_message` | `tgar-telegram::bot` | **P22c** |
| `tg_agent_relay.agent_handle` | `tgar-core::agent_handle` | **P22b** |
| `tg_agent_relay.agent_stamp` | `tgar-core::agent_stamp` | **P22b** |
| `tg_agent_relay.goal_events` | `tgar-core::goal_events` | **P22b** |
| `tg_agent_relay.config` | `tgar-core` | planned |
| `tg_agent_relay.metrics` | `tgar-core` | planned |
| `tg_agent_relay.send` (full EnvSender) | `tgar` + crates | later phases |