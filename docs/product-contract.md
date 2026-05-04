# Product Contract

CueCanvas is a show graphics system for safely preparing, validating,
previewing, and taking live graphics into OBS.

The MVP is plugin-ready but plugin-free. It reserves actor, origin, capability,
and extension metadata fields, but does not execute third-party plugins.

## Invariants

1. Program changes only through Take, Clear, Blackout, Restore, or a future
   explicit LiveCommand.
2. Take promotes an existing `PreviewSnapshot` to an immutable
   `ProgramSnapshot`.
3. Take must not re-resolve a Cue.
4. Editing Cue, Data, Template, Asset, or extension metadata must not implicitly
   change Program.
5. Overlay receives only `OverlayState`, asset/font URLs, sequence, and
   renderer protocol version.
6. Runtime is the source of truth for ProjectPackage, ShowDefinition,
   RunSession, PreviewSnapshot, ProgramSnapshot, Preflight, OBS/Overlay health,
   and Operation Log.
7. Commands, events, and operation log entries include an actor.
8. Major artifacts include origin and preserve unknown extension metadata.
9. Preflight errors may block Live Start and Take.
10. Plugin-created artifacts are represented in data, but plugins cannot write
    Program, OverlayState, or OBS directly.
