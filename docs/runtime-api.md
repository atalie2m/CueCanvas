# Runtime API

The runtime binds to `127.0.0.1` and exposes local-only HTTP and WebSocket
endpoints for the editor, overlay, and host app.

## Implemented In The First Slice

- `GET /health`
- `GET /api/project`
- `GET /api/preflight`
- `GET /api/runtime/health`
- `GET /api/recovery/candidates`
- `POST /api/project/save`
- `POST /api/project/open`
- `POST /api/data/import`
- `POST /api/live/preview/:cueId`
- `POST /api/live/take`
- `POST /api/live/clear`
- `POST /api/live/blackout`
- `POST /api/live/restore`
- `GET /api/overlay/program/snapshot`
- `GET /assets/:assetId`
- `GET /fonts/:fontId`
- `GET /editor`
- `GET /editor/*path`
- `GET /overlay/program`
- `WS /ws/editor`
- `WS /ws/overlay/program`

## Local Tokens

The runtime creates separate local tokens when it starts:

- editor token: required for project reads, package open/save, and all live
  mutation commands.
- overlay token: required for read-only overlay HTML, asset/font, snapshot, and
  WebSocket endpoints.

Editor requests can pass `Authorization: Bearer <editor-token>` or
`X-CueCanvas-Editor-Token: <editor-token>`. Overlay requests can pass
`X-CueCanvas-Overlay-Token: <overlay-token>` or a `?token=<overlay-token>`
query, which is the expected OBS Browser Source form. Overlay asset and font
URLs also use the overlay query token. Overlay credentials are not accepted by
project or live mutation routes.

## Command Rules

Every mutating command must include or derive:

- actor
- expected revision where relevant
- idempotency key for live operations
- target RunSession and OutputTarget
- validation through the Runtime state actor

Overlay routes are read-only and must never expose project mutation commands.

Take is blocked when show or preview preflight produces an Error. The current
slice includes blocking disconnected-output and stale/mismatched-preview checks.

## Data Import Route

`POST /api/data/import` is an editor-token-only route for People, Sessions, and
Sponsors CSV/TSV import. It accepts a typed entity target, source name, format,
raw content, column mappings, and a `replaceExisting` flag:

```json
{
  "entityType": "people",
  "sourceName": "people.csv",
  "format": "csv",
  "content": "name,role\nAda Lovelace,Speaker\n",
  "mappings": [
    { "source": "name", "target": "displayName" },
    { "source": "role", "target": "role" }
  ],
  "replaceExisting": true
}
```

Imports update the typed entity collections and reflect the same rows into the
internal `DataTable` representation. Data import is an edit operation only; it
does not mutate an existing `ProgramSnapshot`.

## Persistence Routes

`POST /api/project/save` and `POST /api/project/open` both accept:

```json
{
  "packageDir": "/path/to/show.cuecanvas"
}
```

The runtime only accepts package directories whose final path component ends in
`.cuecanvas`. Save uses the package writer, including atomic JSON writes,
extension registry output, and autosave copies of the latest run/program
snapshots.

The package writer uses the split MVP package layout:

- root `project.json` contains project metadata, settings, extension metadata,
  and references to show/run files.
- `show-definitions/*.json` stores `ShowDefinition` payloads.
- `run-sessions/*.json` stores `RunSession` payloads.
- `autosave/` stores latest project/show/run files, latest and retained Program
  snapshots, `command-log.jsonl`, and `recovery-info.json`.
- local tokens and OBS passwords are not written to package JSON.

## Preflight Route

`GET /api/preflight` returns grouped editor-facing preflight state with
`errors`, `warnings`, `infos`, scope/source labels, allowlisted fix actions,
warning override keys, value hashes, and override status. Warning overrides are
bound to the current value hash; when the underlying preflight value changes,
the warning is active again.

## WebSocket Behavior

`WS /ws/editor` requires the editor token and broadcasts runtime events such as
`previewChanged` and `programChanged`. The editor treats these as notifications
and reloads authoritative state from Runtime.

`WS /ws/overlay/program` requires the overlay token, marks the Program overlay
as connected, immediately sends the latest `ProgramSnapshot`, and broadcasts
future Program snapshots after Take, Clear, Blackout, or Restore. Overlay
clients send:

```json
{
  "event": "overlay.rendered",
  "programRevision": 1
}
```

Runtime records this as overlay health/ack state.
