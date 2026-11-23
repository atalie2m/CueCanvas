# CueCanvas Proposal v5.2 Complete Edition

## 0. Positioning of This Version

CueCanvas v5.2 preserves the production safety, snapshot safety, and OBS safety introduced in v5.1, while formally adding future extensibility as a design concern.

The core definition of CueCanvas remains unchanged.

```text
CueCanvas:
  A Show Graphics System for safely sending speaker, session, sponsor,
  and alert graphics into OBS using Templates, Data, Cue Sheets,
  Preview / Program, Preflight, and Take operations.

English:
  A show graphics system for safely preparing, validating, previewing,
  and taking live graphics into OBS.
```

The most important new policy in v5.2 is:

```text
Plugin-ready, plugin-free MVP.
```

This means the MVP does not include a plugin loader or plugin marketplace. However, the model, protocol, command layer, operation log, and security boundaries must reserve enough space to safely support future plugins, connectors, template packs, cue generators, preflight rules, and automation.

---

## 0.1 Major Changes from v5.1 to v5.2

```text
1. Add an Extension Readiness Policy
2. Add artifact origin to all major models
3. Add preservation of extensions metadata
4. Add actor to Command / Event / Operation Log
5. Introduce a Capability model
6. Require all plugin contributions to pass Runtime validation
7. Prohibit plugins from directly writing ProgramSnapshot / OverlayState / OBS output
8. Define Declarative Plugin Packs as the first extension target
9. Define Data Importers / Cue Generators / Preflight Rules as mid-term extension targets
10. Treat Renderer Plugins / Native Plugins / OBS Automation as later-stage, high-risk features
11. Add plugin lifecycle, compatibility, migration, security, and sandbox policies
12. Extend the file format for plugin-created artifacts and unknown extension metadata preservation
13. Add plugin actor / source tracking to Runtime API and Command schema
14. Add Extension Readiness and post-MVP Plugin phases to the Roadmap
```

---

## 0.2 Policies Preserved in v5.2

The following policies from v5.1 remain unchanged.

```text
- Keep SwiftUI Host
- Do not use Tauri
- Implement UI with React + TypeScript
- Use a Rust sidecar process as the Runtime
- Runtime is the source of truth
- Editor only holds transient state
- Overlay Renderer knows only OverlayState, not Project / Show / Cue / Slot
- Place one Browser Source in OBS by default
- Treat Cue as declarative state, not an instruction sequence
- MVP supports absolute cues only
- Separate ShowDefinition and RunSession
- Take is a PreviewSnapshot commit
- ProgramSnapshot is immutable
- Editing Cue must not implicitly change Program
- Template slots use a local slot contract
- DataTable is an internal concept; UI presents People / Sessions / Sponsors
- Introduce OBS Setup Wizard early
- Exclude OBS Scene Binding from MVP
- Use the Renderer Confidence Model
- Stabilize Overlay URLs
- External Input is latched by default
- Save as .cuecanvas package with atomic save and autosave
- Board Designer is not core UI; it is a later Advanced Template Editor
```

---

## 1. Product Purpose

CueCanvas is a macOS app for making live graphics operation safe in OBS-based livestreams and recordings.

Primary graphics include:

```text
- Lower Third
- Speaker Card
- Session Title
- Sponsor Bug
- Alert Banner
- Break Screen
- Countdown
- Simple score / status graphic
- Event title graphic
```

OBS is the final output destination. CueCanvas does not replace OBS scene or source management. It sends live graphics state reliably to a single Browser Source in OBS.

The purpose of v5.2 is not merely to create graphics.

```text
The purpose is to provide an operating environment where event operators can
verify the next graphic, validate data and appearance, prevent misfires,
and Take only an approved PreviewSnapshot into Program.
```

v5.2 also prepares the system to safely accept future extensions such as:

```text
- Template packs
- Theme packs
- CSV / TSV mapping presets
- Data importers
- Cue generators
- Preflight rules
- External input providers
- OBS automation helpers
- Renderer component extensions
```

All of these must remain under Runtime validation, capability control, operation logging, and live safety policy.

---

## 1.1 Core Problem to Solve

The essential problem is:

```text
During a live production, show the correct speaker name, title,
session name, photo, and sponsor graphic in OBS without mistakes or delay.
```

Specifically, CueCanvas must solve the following:

```text
- Avoid wrong speaker names
- Avoid wrong titles / roles
- Avoid wrong photos or logos
- Preview the next graphic before it goes live
- Compare Preview and Program before Take
- Always know what is currently on Program
- Detect missing required data before going live
- Detect text overflow before going live
- Restore Program after OBS Browser Source reload
- Prevent live edits from implicitly changing Program
- Prevent Cue / Data changes after Preview from unexpectedly changing Take output
- Prevent external inputs from unexpectedly changing Program
- Prevent plugins or automation from unexpectedly changing Program
- Provide Clear / Blackout / Restore for emergency recovery
- Avoid proliferating sources in OBS
```

---

## 1.2 What CueCanvas Is Not

CueCanvas is not primarily:

```text
- A general whiteboard
- A general design tool
- A Figma replacement
- A Miro / FigJam replacement
- An After Effects replacement
- An OBS replacement
- A full video editing application
- A dedicated broadcast system for large broadcast stations
- A cloud collaboration tool
- A complex 3D compositing tool
- An advanced motion graphics timeline
- A general spreadsheet application
- A general plugin execution environment
- An arbitrary code execution platform
```

CueCanvas provides template editing and layout editing only to the extent required for event graphics operation. Safety, determinism, recoverability, and operating speed take priority over unrestricted freedom.

---

## 2. Product Concept

### 2.1 One-Sentence Definition

```text
A Show Graphics System that safely sends OBS live graphics using Templates,
Data, Cues, Preview, Preflight, and Take, while allowing future extensions
only under Runtime validation.
```

### 2.2 Tagline

```text
Prepare the show.
Check every cue.
Take graphics safely.
Extend without breaking live safety.
```

### 2.3 Core Experience

The v5.2 core experience consists of six steps.

```text
1. Setup:
   Create a ShowDefinition and configure Stage and OBS output

2. Template:
   Choose Templates such as Lower Third, Session Title, Sponsor Bug

3. Data:
   Enter or import typed entities such as People, Sessions, Sponsors

4. Cue:
   Build the Run of Show using the Cue Sheet / Run Sheet

5. Check:
   Detect problems using PreviewSnapshot, Diff, and Preflight

6. Take:
   Commit an approved PreviewSnapshot as a ProgramSnapshot from Live Run View
```

Future extensions must support this core experience rather than bypass it.

```text
Plugin submits a contribution or command request.
Runtime validates it.
Preflight and operator confirmation are required when needed.
```

---

## 3. Target Users

### 3.1 Initial Main Target

```text
Operators of small to mid-sized OBS-based conferences, internal events,
and webinars lasting one or two days.
```

These users strongly need to:

```text
- Avoid mistakes in speaker names and titles
- Preview the next graphic
- Avoid creating many OBS Text Sources / Image Sources
- Link Cues with actual visual output
- Reduce manual spreadsheet-to-OBS synchronization
- Take safely during the live show
- Recover from Browser Source refresh or OBS scene operations
- Recover quickly from mistakes
```

### 3.2 Initial Use Cases

```text
- Lower Third
- Session Title
- Sponsor Bug
- Break Screen
```

Initial data types:

```text
- People
- Sessions
- Sponsors
```

Initial cue constraints:

```text
- Absolute cues only
- One Program Output only
- One OBS Browser Source only
- Operator-driven Preview / Take only
```

### 3.3 Future Extension Users

v5.2 prepares the design for future users such as:

```text
- Event teams that want to distribute custom event templates
- Users who want to sync data from Google Sheets / Airtable
- Developers who want to create Cue generators for competitions or award shows
- Production teams that want custom Preflight rules
- Developers integrating CueCanvas with event management systems
- Advanced users who want staged OBS automation
```

These are not the primary MVP users.

---

## 4. Product Principles

### 4.1 Prioritize Live Safety Over Freedom

CueCanvas prioritizes safe live operation over unrestricted editing.

```text
Free editing matters, but dangerous editing must not be possible during Live.
```

### 4.2 Program Must Not Change Implicitly

Program is the current state being output to OBS.

```text
Operations allowed to change Program:
  - Take
  - Clear
  - Blackout
  - Restore Program
  - Re-take Current
  - Explicit Apply to Program
  - Authorized External Automation LiveCommand, post-MVP only
```

The MVP does not support Program changes through External Automation.

The following must not change Program:

```text
- Cue edit
- Data edit
- Template edit
- Asset replacement
- Preview selection
- PreviewSnapshot creation
- Plugin contribution submission
- External Input update, except post-MVP live input
- OBS scene change, except explicit post-MVP opt-in automation
```

### 4.3 Take Is a PreviewSnapshot Commit

In v5.2, Take is defined as:

```text
Take:
  Promote the currently approved PreviewSnapshot to ProgramSnapshot.
```

Take must not re-resolve the Cue.

### 4.4 Plugin Must Not Directly Change Program

Plugins cannot directly modify ProgramSnapshot, OverlayState, or OBS output.

```text
Allowed:
  Plugin -> contribution / command request -> Runtime validation -> state update

Forbidden:
  Plugin -> direct ProgramSnapshot write
  Plugin -> direct OverlayState write
  Plugin -> direct OBS control
  Plugin -> bypass Preflight
```

### 4.5 Template Is a First-Class Concept

TextNode and ImageNode are internal representations. Users primarily operate on Templates and TemplateInstances.

```text
Good UI:
  Choose a Lower Third Template
  Bind primaryText to Person.displayName
  Bind secondaryText to Person.role
  Switch Person from the Cue Sheet

Avoid:
  Create a TextNode manually
  Write patches against item IDs
  Change image node asset IDs directly
  Put speaker.name directly into Template slot keys
```

### 4.6 Do Not Over-Separate Data and Cue

People, Sessions, and Sponsors are tightly connected to the Cue Sheet.

```text
Typed Entity is not auxiliary data; it is an entry point for cue creation.
```

DataTable may exist internally, but the initial UI should present typed entities, not DataTables.

### 4.7 Preflight Is a Gate

Preflight is not informational decoration.

```text
Error:
  May block Live Start / Take

Warning:
  Can pass with explicit confirmation

Info:
  Logged only
```

Plugin-generated PreflightResult must also be checked for ownership, scope, severity, and capability by the Runtime.

### 4.8 OBS Integration Is Part of the Product Experience

Manually pasting a Browser Source URL should be a fallback path.

```text
Ideal:
  OBS Setup Wizard creates, updates, and verifies the Browser Source

Fallback:
  Copy manual setup URL
```

### 4.9 Use a Renderer Confidence Model, Not a Perfect-Match Claim

Editor Preview and OBS Browser Source are not guaranteed to be identical rendering environments.

```text
Editor Preview:
  High-confidence preview for fast interaction

Chromium Golden Render:
  Regression test / thumbnail / overflow detection

OBS Verified Render:
  Production-equivalent verification inside actual OBS Browser Source
```

### 4.10 Extensibility Exists to Preserve Safety Boundaries

Extensibility in v5.2 is not for arbitrary code execution.

```text
Purpose of extensibility:
  - Add Templates
  - Improve Data import
  - Automate Cue generation
  - Strengthen Preflight
  - Connect existing workflows

Non-goals:
  - Bypass Runtime validation
  - Direct Program manipulation
  - Direct OBS manipulation
  - Become an arbitrary code execution platform
```

---

## 5. Core Concepts

### 5.1 ProjectPackage

ProjectPackage is the save unit.

```text
ProjectPackage:
  The complete project stored as a .cuecanvas package.
```

ProjectPackage contains:

```text
- ProjectMetadata
- ShowDefinitions
- RunSessions
- GlobalAssets
- ExtensionRegistry metadata
- Settings
- Autosave
- Recovery data
```

In the MVP, it is acceptable to use one ProjectPackage, one ShowDefinition, and one active RunSession.

### 5.2 ShowDefinition

ShowDefinition is the prepared show state.

```text
ShowDefinition:
  The definition of one event, stream, recording, webinar, or internal show.
```

ShowDefinition contains:

```text
- Stage
- OutputTargets
- TemplateDefinitions
- TemplateInstances
- Assets
- TypedEntities
- DataTables
- CueSheet
- ShowSettings
- extensions metadata
```

ShowDefinition does not directly own live state.

### 5.3 RunSession

RunSession is execution state.

```text
RunSession:
  The rehearsal or live execution state for a ShowDefinition.
```

RunSession contains:

```text
- runSessionId
- showDefinitionId
- showDefinitionRevision
- liveState
- previewSnapshot
- programSnapshot
- retainedProgramSnapshot
- outputConnectionState
- obsHealth
- overlayHealth
- warningOverrides
- operationLog
- runSettings
- extensions metadata
```

### 5.4 TemplateDefinition

TemplateDefinition is a reusable graphics definition.

TemplateDefinition exposes a local slot contract.

```text
Lower Third Basic:
  primaryText
  secondaryText
  portrait
```

TemplateDefinition does not directly know external data names such as People, Sessions, or Sponsors.

### 5.5 TemplateInstance

TemplateInstance is an instance of a Template placed on the Stage in a ShowDefinition.

Examples:

```text
- Main Lower Third
- Top Right Sponsor Bug
- Center Alert Banner
```

TemplateInstance contains:

```text
- templateDefinitionId
- templateVersionPolicy
- position
- size
- variant
- slotBindings
- visibility policy
- safe area policy
- transition settings
- origin
- extensions metadata
```

### 5.6 Typed Entity

Typed Entity is operational data shown to the user.

MVP typed entities:

```text
People:
  id, displayName, role, organization, photo

Sessions:
  id, title, track, startTime, speakerRefs

Sponsors:
  id, name, logo, tier
```

Internally, these may be stored as DataTables. The UI should present People / Sessions / Sponsors.

### 5.7 DataTable

DataTable is the internal storage representation for typed entities.

```text
DataTable:
  The backing structure for CSV import, TSV paste, manual input,
  and future external integrations.
```

The user-facing UI should not foreground the term DataTable.

### 5.8 Cue

Cue is a declarative state that can be Previewed and optionally Taken into Program at a particular moment.

A v5.2 Cue contains:

```text
- cue number
- segment
- cue name
- cue type
- entity references
- local slot overrides
- template visibility state
- optional item patches
- transition hints
- operator notes
- preflight status
- origin
- extensions metadata
```

### 5.9 PreviewSnapshot

PreviewSnapshot is an OverlayState snapshot generated as a candidate for the next Take.

```text
PreviewSnapshot:
  - previewSnapshotId
  - previewRevision
  - sourceCueId
  - resolvedData
  - overlayState
  - preflightResult
  - source origin chain
  - createdAt
```

PreviewSnapshot freezes the resolved result at creation time.

### 5.10 ProgramSnapshot

ProgramSnapshot is an immutable output artifact created at Take time.

```text
ProgramSnapshot:
  - programSnapshotId
  - programRevision
  - sourcePreviewSnapshotId
  - sourceCueId
  - resolvedData
  - overlayState
  - assetManifest
  - origin chain
  - createdAt
  - outputTargetId
```

### 5.11 Origin

Origin describes who or what created an artifact.

```json
{
  "kind": "plugin",
  "pluginId": "com.example.cue-generator",
  "pluginVersion": "1.2.0"
}
```

Origin kinds:

```text
user:
  Created by user action

system:
  Generated by CueCanvas core

plugin:
  Generated by a plugin or extension

import:
  Generated by CSV / TSV / external import

migration:
  Generated or modified by migration
```

Applicable targets:

```text
- TemplateDefinition
- TemplateInstance
- Cue
- TypedEntity
- DataTable
- PreflightResult
- Asset
- PreviewSnapshot
- ProgramSnapshot
```

### 5.12 Extensions Metadata

Major models preserve unknown extension metadata.

```json
{
  "extensions": {
    "com.example.plugin": {
      "customKey": "customValue"
    }
  }
}
```

Policy:

```text
- CueCanvas core must not discard unknown extension metadata
- Save must preserve it through round-trip
- Removing a plugin must not cause data loss
- Unknown extension metadata is not semantically validated by Runtime,
  but namespace validation and size limits still apply
```

---

## 6. Product Contract

v5.2 defines the following as non-negotiable product invariants.

```text
1. Program changes only through Take / Clear / Blackout / Restore / explicit LiveCommand
2. OBS scene changes and external automation do not change Program in the MVP
3. Take promotes PreviewSnapshot to ProgramSnapshot
4. Take does not re-resolve Cue
5. Direct Cue Preview resolution is deterministic
6. ProgramSnapshot is immutable
7. PreviewSnapshot preserves resolved state at creation time
8. Overlay does not know Project / ShowDefinition / Cue / DataTable
9. Runtime sends the latest ProgramSnapshot when Overlay reconnects
10. Editor is not a source of truth
11. Dangerous live edits are blocked by the state machine
12. Preflight Error may block Live Start / Take
13. OBS Browser Source settings are covered by Setup Wizard / Health Check
14. Renderer preview uses a confidence model and is not described as exact guarantee
15. Plugin must not directly mutate Runtime state
16. Plugin must not directly modify ProgramSnapshot / OverlayState / OBS output
17. Plugin contribution must pass Runtime validation
18. Command / Event / Operation Log must have actor
19. Plugin-created artifact must have origin
20. Unknown extension metadata must be preserved on save
```

---

## 7. Extension Readiness Policy

### 7.1 Basic Policy

The v5.2 MVP does not implement a plugin loader.

However, the following must be reserved in the model, protocol, and storage for future plugins:

```text
- artifact origin
- extensions metadata
- command actor
- event actor
- operation log actor
- capability model
- plugin-created PreflightResult
- plugin-created TemplateDefinition / Cue / Entity identification
- unknown extension metadata preservation
```

### 7.2 What Plugins Can Do

Plugins may submit contributions or command requests to the Runtime.

Examples:

```text
- Add TemplateDefinition
- Add Theme preset
- Add CSV mapping preset
- Import People / Sessions / Sponsors
- Generate Cues
- Generate PreflightResult
- Provide ExternalInput value
```

All submissions must pass Runtime validation.

### 7.3 What Plugins Cannot Do

```text
- Directly write ProgramSnapshot
- Directly write OverlayState
- Directly control OBS
- Bypass Preflight
- Bypass the Live State Machine
- Bypass Runtime revision checks
- Destructively write project files directly
- Steal Editor session token
- Use Overlay token to access Project API
```

### 7.4 Plugin-Ready but Plugin-Free MVP

Included in the MVP:

```text
- origin field
- extensions metadata field
- actor field in Command / Event / OperationLog
- capability schema draft
- PreflightResult source field
- UI readiness for plugin-created artifacts
- unknown extension metadata round-trip preservation
```

Excluded from the MVP:

```text
- plugin loader
- plugin marketplace
- native plugin execution
- sandboxed JS / WASM execution
- renderer plugin execution
- plugin installation UI
- third-party plugin signing / distribution
```

---

## 8. Extension Types

### 8.1 Level 1: Declarative Plugin Pack

This should be the first supported extension type.

```text
- Template pack
- Theme pack
- Asset pack
- CSV mapping preset
- Show starter template
```

Characteristics:

```text
- No code execution
- Manifest + JSON + assets only
- Runtime validates schema
- High safety
```

Expected package layout:

```text
.cuecanvas-plugin/
├── manifest.json
├── templates/
├── assets/
├── mappings/
└── presets/
```

### 8.2 Level 2: External Connector Plugin

The next extension type to support.

```text
- Google Sheets importer
- Airtable importer
- JSON feed importer
- Speaker database importer
- Event management system importer
```

Characteristics:

```text
- Runs as a separate process or external application
- Sends command requests to Runtime API
- Requires OAuth / API key / token management
- Must have limited write capabilities
```

### 8.3 Level 3: Sandboxed Logic Plugin

Mid- to long-term extension type.

```text
- Cue generator
- Preflight rule
- Data transformer
- Title normalizer
- Asset naming checker
```

Possible execution modes:

```text
- WASM
- Sandboxed JavaScript worker
- Separate process with restricted Runtime API
```

Policy:

```text
- Timeout required
- Memory limit required
- Network access disabled by default or capability-gated
- File access disabled by default
- Direct Runtime state access prohibited
```

### 8.4 Level 4: Renderer Extension

A later-stage, high-risk extension type.

```text
- Custom visual component
- Chart renderer
- Scoreboard widget
- Data-driven graphic component
```

Risks:

```text
- OBS rendering differences
- CSS breakage
- Performance degradation
- Font fallback issues
- Security
- More complex screenshot testing
```

Policy:

```text
- Not included in MVP
- Start with an allowlisted component API after MVP
- Do not allow arbitrary HTML / JS injection
- Must be covered by OBS Verified Render
```

### 8.5 Level 5: Native Plugin

Generally not recommended.

```text
- Rust dylib
- Swift plugin bundle
- Node native addon
```

Reasons:

```text
- Runtime crashes
- Signing / notarization complexity
- ABI compatibility
- Weak security boundary
- Plugin API versioning burden
```

Native Plugin is out of scope for v5.2.

---

## 9. Plugin Manifest

### 9.1 Manifest Example

```json
{
  "manifestVersion": 1,
  "pluginId": "com.example.event-pack",
  "name": "Example Event Pack",
  "version": "1.0.0",
  "publisher": "Example Studio",
  "kind": "declarativePack",
  "minimumCueCanvasVersion": "0.3.0",
  "capabilities": [
    "contribute.templates",
    "contribute.assets",
    "contribute.csvMappings"
  ],
  "contributes": {
    "templates": ["templates/lower-third.json"],
    "assets": ["assets/example-logo.png"],
    "csvMappings": ["mappings/speakers-basic.json"]
  }
}
```

### 9.2 Manifest Validation

Runtime validates:

```text
- pluginId format
- manifestVersion
- semantic version
- minimumCueCanvasVersion
- consistency between capabilities and contributes
- no file path traversal
- asset size limits
- schema validation
- rejection of unsupported contributions
```

### 9.3 Plugin Identity

Plugin identity consists of:

```text
pluginId
pluginVersion
publisher
signature status, post-MVP
installationId, post-MVP
```

The MVP does not implement a plugin loader, but the origin schema must support pluginId / pluginVersion.

---

## 10. Capability Model

### 10.1 Basic Policy

Plugins declare capabilities. Runtime enforces the allowed scope per capability.

Example:

```json
{
  "pluginId": "com.example.google-sheets-importer",
  "capabilities": [
    "read.showDefinition",
    "write.typedEntities",
    "create.cues",
    "emit.preflightResults"
  ]
}
```

### 10.2 Capability Categories

```text
read.*:
  Read state

write.*:
  Write to editable targets

create.*:
  Create artifacts

contribute.*:
  Add declarative contributions

emit.*:
  Emit events or PreflightResults

external.*:
  Connect to network or external services

automation.*:
  Participate in LiveCommand or OBS automation
```

### 10.3 Capabilities Reserved in MVP

```text
contribute.templates
contribute.assets
contribute.csvMappings
contribute.showStarters

write.typedEntities
create.cues
emit.preflightResults

read.showDefinition
read.runSession
```

### 10.4 Capabilities Forbidden from the Start

```text
direct.program.write
direct.overlay.write
direct.obs.control
bypass.preflight
bypass.runtime.validation
bypass.liveStateMachine
read.editorSessionToken
read.overlayToken
write.projectFileDirectly
```

### 10.5 Dangerous Capabilities

The following remain warning-gated even after MVP:

```text
automation.previewCue
automation.liveTake
automation.clear
automation.blackout
automation.obsSceneBinding
external.network
external.fileRead
external.fileWrite
```

---

## 11. Command / Event / Actor

### 11.1 Actor

Every Command has an actor.

```json
{
  "command": "cue.create",
  "actor": {
    "kind": "plugin",
    "pluginId": "com.example.cue-generator",
    "pluginVersion": "1.2.0"
  },
  "payload": {}
}
```

Actor kinds:

```text
user
system
plugin
externalInput
migration
recovery
```

### 11.2 Command Validation

Runtime validates:

```text
- actor identity
- capability
- expectedRevision
- idempotency key
- LiveState policy
- Preflight blocking policy
- payload schema
- target existence
- project / runSession consistency
```

### 11.3 Operation Log

Live operations and plugin actions are recorded in the Operation Log.

Example:

```text
12:01:04 User entered Live Armed
12:01:10 Plugin com.example.cue-generator created 12 cues
12:01:11 User previewed Cue 002
12:01:12 User took PreviewSnapshot 018
12:03:10 OBS connection degraded
12:03:12 User restored ProgramSnapshot 042
```

### 11.4 Event Source

Events also include source.

```json
{
  "event": "data.changed",
  "source": {
    "kind": "plugin",
    "pluginId": "com.example.importer"
  },
  "revision": 42,
  "payload": {}
}
```

---

## 12. Plugin Contribution Model

### 12.1 Basic Policy

Plugin does not directly mutate Runtime state.

```text
Plugin
  -> contribution / command request
  -> Runtime validation
  -> state update
  -> derived state rebuild
  -> Preflight
  -> Event broadcast
  -> Operation Log
```

### 12.2 Contribution Types

```text
templateDefinition:
  Added by Template pack

themePreset:
  Color, font, spacing presets

csvMappingPreset:
  Import column mapping

showStarter:
  Starter ShowDefinition

typedEntityImport:
  People / Sessions / Sponsors import

cueGeneration:
  Cue candidate generation

preflightResult:
  Plugin-generated PreflightResult

externalInputValue:
  External value update
```

### 12.3 Contribution Lifecycle

```text
submitted
validated
accepted
rejected
applied
superseded
removed
```

### 12.4 Plugin-Created Artifact

Plugin-created artifacts have origin.

```json
{
  "id": "cue-generated-001",
  "origin": {
    "kind": "plugin",
    "pluginId": "com.example.cue-generator",
    "pluginVersion": "1.2.0"
  }
}
```

### 12.5 Plugin Removal

Removing a plugin must not delete existing artifacts automatically.

```text
- plugin-created Cue remains
- plugin-created TemplateDefinition remains or shows missing plugin warning
- extension metadata is preserved
- plugin dependency warning appears in Preflight
```

---

## 13. Core Workflows

### 13.1 New Show Workflow

```text
1. Create New Show
2. Select Stage preset
   - 1920x1080
   - 1280x720
   - custom
3. Open OBS Setup Wizard
4. Connect to OBS
5. Create or update Browser Source
6. Verify Browser Source URL / size / CSS / refresh / shutdown settings
7. Confirm Program Overlay connection
8. Show Test Pattern
9. Choose initial Templates from Template Library
10. Prepare People / Sessions / Sponsors
11. Create Cue Sheet
```

If Declarative Plugin Packs exist in the future, their contributions appear in Template Library, Show Starter, and CSV Mapping.

### 13.2 Template Setup Workflow

```text
1. Open Template Library
2. Choose Lower Third Basic
3. Select position on Stage
4. Choose variant
5. Confirm local slots
6. Bind primaryText / secondaryText / portrait to entity fields
7. Configure safe area and overflow policy
8. Confirm in Preview
```

In the MVP, users do not edit template internals freely. They edit TemplateInstance position, size, variant, and slot bindings.

### 13.3 Data Import Workflow

```text
1. Open People / Sessions / Sponsors
2. Import CSV or paste TSV
3. Map columns
   name  -> Person.displayName
   role  -> Person.role
   photo -> Person.photo
4. Validate required columns
5. Detect missing images
6. Show long text warnings
7. Save as typed entity
8. Reflect internally into DataTable
```

Future Data Importer plugins enter before this workflow.

```text
External source
  -> importer plugin
  -> typedEntityImport contribution
  -> Runtime validation
  -> Preview import diff
  -> User apply
```

### 13.4 Cue Preparation Workflow

```text
1. Open Cue Sheet
2. Choose Add Cue or Generate Cues from Data
3. Select Template and entity row for each Cue
4. Confirm local slot bindings
5. Enter Cue-specific slot overrides
6. Generate Cue thumbnail
7. Check Cue diff and Preflight status
```

Future Cue Generator plugins generate Cue candidates only. They do not change Program.

### 13.5 Live Workflow

```text
1. Pass Preflight
2. Enter Live Armed
3. Send Cue to Preview
4. Generate PreviewSnapshot
5. Check Visual Preview
6. Check Diff Preview
7. Take
8. PreviewSnapshot becomes ProgramSnapshot
9. ProgramRevision increments
10. Full snapshot is sent to OBS Program Overlay
11. Overlay ACKs programRevision
12. Operation Log records the action
```

Plugins cannot bypass Take.

---

## 14. UX Modes

### 14.1 Setup Mode

Purpose:

```text
Configure ShowDefinition, Stage, OBS output, and initial Templates.
```

Main UI:

```text
- New Show Wizard
- Stage Preset
- OBS Setup Wizard
- Output Target Status
- Template Starter Picker
- Test Pattern
```

### 14.2 Template Mode

Purpose:

```text
Place and configure TemplateInstances.
```

Main UI:

```text
- Stage View
- Template Library
- Template Instance List
- Inspector
- Slot Binding Panel
- Safe Area Preview
- Variant Picker
```

Future extension UI:

```text
- plugin-provided Template badge
- source / origin display
- missing plugin warning
```

### 14.3 Data Mode

Purpose:

```text
Manage operational data used by Cues.
```

Main UI:

```text
- People editor
- Sessions editor
- Sponsors editor
- CSV import
- TSV paste
- Column mapping
- Asset resolver
- Validation panel
```

Future extension UI:

```text
- Import from connector
- Mapping preset from plugin
- Imported row origin
- Sync stale warning
```

### 14.4 Cue Mode

Purpose:

```text
Prepare Cues as a Run Sheet.
```

Main UI:

```text
- Cue Sheet / Run Sheet
- Cue Inspector
- Thumbnail Preview
- Entity row selector
- Template selector
- Slot override editor
- Diff Preview
- Preflight status
```

Future extension UI:

```text
- Generate Cues action
- plugin-created Cue badge
- generation source
- regenerate / detach from plugin
```

### 14.5 Rehearsal Mode

Purpose:

```text
Check Cues in a production-like flow before the live show.
```

Main UI:

```text
- Cue List
- Preview monitor
- Program monitor
- Diff Preview
- Take rehearsal
- Preflight panel
- Operation log
- Overlay health
- OBS health
```

### 14.6 Live Mode

Purpose:

```text
Safely Preview / Take / Recover during the live show.
```

Main UI:

```text
- Run View
- Current Program
- Next Preview
- Compact Cue List
- Diff Preview
- Take button
- Clear
- Blackout
- Restore Program
- OBS status
- Overlay status
- Operation log
```

Operations restricted in Live Mode:

```text
- Destructive TemplateDefinition change
- Stage size change
- OutputTarget deletion
- Delete TemplateInstance currently used by Program
- Data schema change
- Dangerous bulk CueStack edit
- Destructive asset deletion
- Dangerous plugin action
- Plugin install / uninstall
- Plugin capability change
```

---

## 15. Live State Machine

### 15.1 States

```text
Setup:
  ShowDefinition preparation. Live operations unavailable.

Rehearsal:
  Cues can be tested in a production-like flow.

PreflightReady:
  No blocking errors.

LiveArmed:
  Immediately before going live. Take available.

LiveRunning:
  Live show in progress. Dangerous edits are restricted.

OutputDegraded:
  OBS / Overlay / Runtime has a partial problem.

Recovering:
  Recovery in progress.

Ended:
  After live show.
```

### 15.2 Take Conditions

```text
Take available when:
  - PreviewSnapshot exists
  - PreviewSnapshot belongs to the current RunSession
  - previewRevision matches Runtime previewRevision
  - expectedProgramRevision matches current programRevision, or override is explicit
  - OutputTarget is connected, or warning is explicitly acknowledged
  - PreviewSnapshot has no blocking preflight error
  - Runtime revision is consistent
  - actor has live.take capability; MVP allows user only
```

### 15.3 Live Plugin Policy

MVP:

```text
- No plugin execution
- No plugin install / uninstall
```

Post-MVP:

```text
Allowed:
  - read-only Preflight rule
  - preview-only Cue suggestion
  - non-program data import draft

Allowed with warning:
  - next Cue generation
  - non-program entity update

Forbidden:
  - direct Program update
  - destructive TemplateDefinition update
  - OutputTarget update
  - direct OBS control
  - capability change
```

---

## 16. Take Semantics

### 16.1 Basic Policy

v5.2 Take does not apply a Cue directly to Program.

```text
Cue
  -> resolve
  -> PreviewSnapshot
  -> operator confirms
  -> Take
  -> ProgramSnapshot
```

### 16.2 live.take Command

```json
{
  "command": "live.take",
  "actor": {
    "kind": "user",
    "userId": "local-user"
  },
  "takeIntentId": "take-uuid-001",
  "runSessionId": "run-2026-05-04-main",
  "outputTargetId": "output-program-obs",
  "previewSnapshotId": "preview-123",
  "previewRevision": 18,
  "expectedProgramRevision": 42
}
```

If plugin / automation requests live.take after MVP, Runtime validation and explicit policy still apply.

### 16.3 Idempotency

Take, Clear, Blackout, and Restore require an idempotency key.

```text
If Runtime receives the same takeIntentId multiple times,
it returns the same result and does not update Program twice.
```

### 16.4 Preview Stale Handling

If Cue / Data / Template is edited after Preview, the PreviewSnapshot itself does not change.

UI message:

```text
Preview was generated from Cue 002 at revision 18.
Cue 002 has changed since this preview.
Take will use the current PreviewSnapshot, not the edited Cue.
```

Options:

```text
- Take current PreviewSnapshot
- Refresh Preview
- Cancel
```

---

## 17. Preflight Check

### 17.1 Role

Preflight is the safety mechanism for detecting problems before going live and before Take.

v5.2 treats Preflight as a Live readiness gate.

### 17.2 Severity

```text
Error:
  May block Live Start / Take

Warning:
  Can pass with explicit confirmation

Info:
  Logged only
```

### 17.3 Error Examples

```text
- required local slot missing
- slot binding missing
- missing asset
- missing template definition
- missing template version
- Program overlay not connected
- OBS Browser Source size mismatch
- Stage size mismatch
- OutputTarget token mismatch
- TemplateInstance references deleted slot
- Cue references missing entity row
- PreviewSnapshot session mismatch
- Plugin-created artifact references missing plugin resource
```

### 17.4 Warning Examples

```text
- text may overflow
- title is longer than recommended maxLength
- image resolution is too low
- font fallback is used
- OBS source refresh setting may cause reload
- OBS shutdown source when not visible may interrupt overlay recovery
- Cue has no operator note in required segment
- Program snapshot diverged from edited Cue
- PreviewSnapshot is stale relative to current Cue
- Plugin-created Cue may be stale relative to source import
- Missing plugin metadata, artifact preserved
```

### 17.5 Plugin PreflightResult

Plugin-generated PreflightResult includes source.

```json
{
  "ruleId": "plugin.com.example.long-title",
  "source": {
    "kind": "plugin",
    "pluginId": "com.example.event-checks",
    "pluginVersion": "1.0.0"
  },
  "severity": "warning",
  "scope": "cue",
  "cueId": "cue-002",
  "message": "Session title is too long for sponsor layout"
}
```

Runtime validates:

```text
- plugin has emit.preflightResults capability
- ruleId namespace matches pluginId
- severity escalation policy is not violated
- scope exists
- message size is within limit
```

### 17.6 Fix Action

Preflight results may include fix actions.

```json
{
  "ruleId": "asset.missing",
  "severity": "error",
  "scope": "cue",
  "cueId": "cue-002",
  "message": "portrait asset is missing",
  "fixActions": [
    { "kind": "openAssetResolver" },
    { "kind": "clearOptionalSlot" }
  ]
}
```

Plugin-provided fix actions are limited to allowlisted action kinds.

### 17.7 Warning Override

Warning override is bound to a value hash.

```text
Override title overflow warning
→ show warning again if title changes
```

---

## 18. Cue Diff Preview

### 18.1 Purpose

Before Take, explicitly show what changes from the current ProgramSnapshot to the PreviewSnapshot.

### 18.2 Example

```text
Next Take changes:

Template:
  Main Lower Third: hidden -> visible

Person:
  primaryText: Hanako Sato -> Taro Yamada
  secondaryText: CTO -> CEO
  portrait: sato.png -> yamada.png

Session:
  title: Opening -> Product Keynote

Origin:
  Cue generated by com.example.cue-generator 1.2.0

Warnings:
  primaryText may overflow
```

### 18.3 Diff Categories

```text
- entity diff
- local slot value diff
- template visibility diff
- asset diff
- style diff
- output state diff
- warning diff
- snapshot revision diff
- origin / plugin source diff
```

---

## 19. Template System

### 19.1 Basic Policy

Template is central to CueCanvas v5.2.

Users first choose a Template, then bind Data to local slots.

### 19.2 TemplateDefinition

TemplateDefinition does not know external data structures.

Example:

```json
{
  "id": "template-lower-third-basic",
  "name": "Basic Lower Third",
  "kind": "lowerThird",
  "version": 1,
  "origin": {
    "kind": "system"
  },
  "localSlots": [
    {
      "key": "primaryText",
      "kind": "text",
      "label": "Primary Text",
      "required": true,
      "maxLength": 32,
      "fallback": "Guest"
    },
    {
      "key": "secondaryText",
      "kind": "text",
      "label": "Secondary Text",
      "required": false,
      "maxLength": 48
    },
    {
      "key": "portrait",
      "kind": "asset.image",
      "label": "Portrait",
      "required": false
    }
  ],
  "variants": [
    { "id": "default", "name": "Default" },
    { "id": "compact", "name": "Compact" }
  ],
  "extensions": {}
}
```

### 19.3 Template Pack

Declarative Plugin Pack may provide TemplateDefinitions.

```text
Plugin TemplateDefinition:
  - schema validation required
  - origin required
  - local slot contract required
  - arbitrary script forbidden
  - asset path validation required
```

### 19.4 Template Version Policy

Updating TemplateDefinition may change existing Cue Preview results.

TemplateInstance has a version policy.

```text
latestCompatible:
  Use the latest compatible version.
  MVP default.

pinned:
  Use a specified version.
  Later feature.
```

UI must be able to show:

```text
Cue preview changed because template version changed.
```

### 19.5 MVP Templates

MVP templates can be few.

```text
- Lower Third Basic
- Session Title
- Speaker Card
- Sponsor Bug
- Break Screen
```

Alert Banner can be late-MVP or post-MVP.

### 19.6 Advanced Template Editor

Later feature.

```text
- Free placement of TextNode / ImageNode / ShapeNode
- group
- component
- exposed property
- variant editor
- transition editor
- responsive layout
```

---

## 20. Data Model

### 20.1 Basic Policy

v5.2 separates ProjectPackage / ShowDefinition / RunSession.

```text
ProjectPackage:
  Save unit.

ShowDefinition:
  Prepared show state.

RunSession:
  Rehearsal or live execution state.
```

MVP may use one ProjectPackage, one ShowDefinition, and one RunSession.

### 20.2 ProjectPackage Hierarchy

```text
ProjectPackage
├── metadata
├── showDefinitions
│   └── ShowDefinition
├── runSessions
│   └── RunSession
├── extensionRegistry
├── globalAssets
└── settings
```

### 20.3 ShowDefinition Hierarchy

```text
ShowDefinition
├── metadata
├── stage
├── outputTargets
├── assets
├── templateDefinitions
├── templateInstances
├── typedEntities
│   ├── people
│   ├── sessions
│   └── sponsors
├── dataTables
├── cueSheet
├── showSettings
└── extensions
```

### 20.4 RunSession Hierarchy

```text
RunSession
├── metadata
├── showDefinitionId
├── showDefinitionRevision
├── liveState
├── previewSnapshot
├── programSnapshot
├── retainedProgramSnapshot
├── outputConnectionState
├── obsHealth
├── overlayHealth
├── warningOverrides
├── operationLog
├── runSettings
└── extensions
```

### 20.5 ProjectPackage Example

```json
{
  "formatVersion": 5,
  "migrationsApplied": [
    "v1-initial",
    "v2-slots-components",
    "v3-show-model",
    "v4-run-session-split",
    "v5-extension-readiness"
  ],
  "metadata": {
    "id": "project-001",
    "name": "Event Graphics",
    "createdAt": "2026-05-04T00:00:00Z",
    "updatedAt": "2026-05-04T00:00:00Z"
  },
  "showDefinitions": [],
  "runSessions": [],
  "extensionRegistry": {
    "installedPlugins": [],
    "knownOrigins": []
  },
  "settings": {},
  "extensions": {}
}
```

### 20.6 Cue Example

```json
{
  "id": "cue-002",
  "number": "002",
  "segment": "Talk 1",
  "name": "CTO / Cat Lower Third",
  "mode": "absolute",
  "cueType": "lowerThird",
  "origin": {
    "kind": "user"
  },
  "entityRefs": {
    "person": {
      "entityType": "people",
      "entityId": "person-cat"
    },
    "session": {
      "entityType": "sessions",
      "entityId": "session-tech"
    }
  },
  "slotOverrides": {
    "instance-main-lower-third": {
      "secondaryText": "CTO"
    }
  },
  "templateStates": [
    {
      "templateInstanceId": "instance-main-lower-third",
      "visible": true
    }
  ],
  "itemPatches": [],
  "transition": {
    "kind": "cut"
  },
  "metadata": {
    "color": "blue",
    "notes": "Bring up after speaker walks on stage"
  },
  "extensions": {}
}
```

### 20.7 Resolution Order

```text
1. Template default
2. Local slot fallback
3. TemplateInstance slotBinding
4. Entity row values from Cue entityRefs
5. Cue slotOverrides
6. ExternalInput value according to mode
7. itemPatches
8. Live manual override
9. Clear / Blackout
```

Plugin contributions enter this order only after being converted into the normal model.

---

## 21. PreviewSnapshot / ProgramSnapshot

### 21.1 PreviewSnapshot

PreviewSnapshot is the Take candidate generated from Cue resolution.

```json
{
  "id": "preview-snapshot-018",
  "runSessionId": "run-2026-05-04-main",
  "showDefinitionId": "show-main",
  "showDefinitionRevision": 12,
  "sourceCueId": "cue-002",
  "previewRevision": 18,
  "createdAt": "2026-05-04T09:59:58Z",
  "originChain": [{ "kind": "user" }],
  "resolvedData": {
    "primaryText": "Cat",
    "secondaryText": "CTO",
    "portrait": "asset-cat-photo"
  },
  "overlayState": {},
  "preflightResult": {}
}
```

### 21.2 ProgramSnapshot

Program is not a reference to a Cue ID. It is an OverlayState snapshot created at Take time.

```text
Take:
  Commit PreviewSnapshot as ProgramSnapshot
```

### 21.3 Program Immutability

ProgramSnapshot is never modified after creation.

Edits to Cue, People, Sessions, Sponsors, TemplateDefinition, TemplateInstance, or Plugin metadata do not change ProgramSnapshot.

### 21.4 ProgramSnapshot Persistence Policy

To support Runtime crash recovery:

```text
- Save latest ProgramSnapshot to autosave
- Save latest retainedProgramSnapshot as well
- Present recovery candidates on Runtime restart
- Let user setting decide automatic recovery behavior
```

---

## 22. OverlayState

### 22.1 Role

OverlayState is the final state rendered by the Renderer.

Overlay Renderer does not know:

```text
- ProjectPackage
- ShowDefinition
- RunSession
- Template
- Cue
- Slot
- DataTable
- Typed Entity
- External Input
- Plugin
```

Overlay Renderer only knows:

```text
- OverlayState
- Asset URL
- Font URL
- Renderer protocol version
```

### 22.2 Delivery Mode

MVP uses full snapshot delivery.

```text
program.changed:
  sequence
  full OverlayState
```

Reasons:

```text
- Robust to reconnect
- Robust to missed state updates
- Avoids delta ordering problems
- Text / Image payloads are small enough
```

### 22.3 Overlay Reconnect

```text
Overlay load
→ WebSocket connect
→ overlay.hello
→ Runtime sends overlay.snapshot
→ Overlay sends overlay.rendered after render completes
→ Overlay receives overlay.changed afterwards
```

### 22.4 Renderer Extension Policy

Renderer Extension is not in the MVP.

If considered post-MVP:

```text
- Arbitrary HTML / JS injection is forbidden
- Use only an allowlisted component API
- Increase protocol version when adding component kind to OverlayState schema
- Cover with Chromium Golden Render and OBS Verified Render
- Provide fallback so one failing plugin component does not blank the whole overlay
```

---

## 23. Renderer Confidence Model

### 23.1 Basic Policy

CueCanvas does not unconditionally guarantee exact matching between Editor Preview and OBS output.

Instead, rendering verification has three layers.

```text
1. Editor Preview
2. Chromium Golden Render
3. OBS Verified Render
```

### 23.2 Editor Preview

```text
Purpose:
  Fast visual confirmation during editing.

Nature:
  High-confidence preview.
  Not a perfect-match guarantee.
```

### 23.3 Chromium Golden Render

```text
Purpose:
  Regression tests, thumbnail generation, overflow detection.

Targets:
  OverlayState fixture
  font fallback fixture
  CJK line breaking fixture
  image fit fixture
  transparent background fixture
  plugin template fixture, post-MVP
```

### 23.4 OBS Verified Render

```text
Purpose:
  Verify production-equivalent output in actual OBS Browser Source.

Targets:
  Browser Source refresh
  source hide/show
  scene switch
  overlay reconnect
  stage size mismatch
  renderer extension, post-MVP
```

### 23.5 Preview Wording Policy

Avoid these UI claims:

```text
- Perfect match
- Exact OBS render
- Guaranteed output
```

Preferred wording:

```text
- High-confidence preview
- OBS verified when connected
- Last OBS render acknowledged
```

---

## 24. OBS Integration

### 24.1 Basic Policy

By default, place one Browser Source in OBS.

Recommended placement:

```text
OBS Scene: CueCanvas Graphics
  └── Browser Source: CueCanvas Program

Each Program Scene:
  └── Scene Source: CueCanvas Graphics
```

This allows each OBS Scene to reuse the same CueCanvas overlay.

### 24.2 OBS Setup Wizard

v5.2 introduces OBS Setup Wizard early.

Wizard actions:

```text
1. Connect to OBS WebSocket
2. Authenticate
3. Select Scene
4. Create or select CueCanvas Graphics scene
5. Create or select Browser Source
6. Set URL
7. Set width / height
8. Set recommended custom CSS
9. Detect risk in shutdown source when not visible
10. Detect risk in refresh browser source when scene becomes active
11. Confirm nested scene placement in Program Scenes
12. Confirm Program Overlay connection
13. Show Test Pattern
14. Show Health Check result
```

### 24.3 Browser Source Health Check

Runtime monitors OBS Source state.

```text
- OBS connected
- target scene exists
- browser source exists
- source URL matches
- width / height matches Stage
- custom CSS is compatible
- shutdown setting is safe or acknowledged
- refresh setting is safe or acknowledged
- source visible in intended scene path
- overlay websocket connected
- latest programRevision rendered
```

### 24.4 Three Output State Layers

```text
ProgramCommitted:
  Runtime committed ProgramSnapshot.

OverlayRendered:
  Overlay inside OBS Browser Source received programRevision and ACKed render complete.

OBSHealthy:
  OBS source exists, is visible, and URL / size / scene path are as expected.
```

### 24.5 OBS Automation Plugin Policy

MVP does not implement OBS automation plugins.

If added post-MVP:

```text
- Plugin does not control OBS directly
- All access goes through Runtime OBS bridge
- Capability required
- LiveState policy required
- Operation Log required
- Preflight required
- Program changes must go through LiveCommand
```

---

## 25. OBS Scene Binding

### 25.1 v5.2 Policy

OBS Scene Binding is excluded from MVP.

Reasons:

```text
- It can create unintended Program changes
- It can conflict with CueCanvas live safety principles
- Operator-driven Preview / Take must be completed first
```

### 25.2 Post-MVP Introduction Order

```text
Post-MVP 1:
  OBS scene change -> CueCanvas context / active scene context only

Post-MVP 2:
  OBS scene change -> Preview change

Post-MVP 3:
  OBS scene change -> armed preview change

Post-MVP 4:
  OBS scene change -> Program update
  Requires explicit opt-in, LiveRunning only, debounce, anti-loop, operation log

Post-MVP 5:
  autoTakeCue
  Treat as advanced feature with production policy
```

### 25.3 Relationship with Plugins

Scene Binding is Runtime-managed automation, not a plugin.

Plugins may provide binding presets, but Program updates must always pass Runtime validation and user opt-in.

---

## 26. Architecture

### 26.1 Overall Architecture

```text
[macOS Host App]
  SwiftUI
  WKWebView
  Native menu
  File picker
  Keychain
  Runtime launcher / monitor
        |
        v
[Rust Runtime]
  ProjectPackage state
  ShowDefinition state
  RunSession state
  TypedEntities / DataTables
  CueEngine
  TemplateResolver
  SlotBindingResolver
  OverlayStateBuilder
  PreflightEngine
  ProgramSnapshotStore
  ExtensionRegistry metadata
  ContributionValidator
  CapabilityPolicy
  Project save/load
  Autosave / recovery
  Local HTTP/WebSocket server
  OBS bridge
        |
        ├── [Editor WebView]
        │      React + TypeScript
        │      Setup Wizard
        │      Template Mode
        │      Data Mode
        │      Cue Sheet
        │      Rehearsal View
        │      Live Run View
        │
        ├── [Embedded Overlay Preview]
        │      Same overlay renderer
        │      /overlay/preview
        │      /overlay/program
        │
        ├── [OBS Browser Source]
        │      HTML/CSS/TypeScript overlay
        │      /overlay/program
        │
        └── [External Apps / Future Plugins]
               External Input API
               Connector API, post-MVP
```

### 26.2 Responsibility Split

| Area                  | Technology         | Responsibility                                                       |
| --------------------- | ------------------ | -------------------------------------------------------------------- |
| macOS Host            | SwiftUI            | Window, Menu, File, Keychain, Runtime monitoring                     |
| Editor                | React + TypeScript | Setup, Template, Data, Cue, Live UI                                  |
| Runtime               | Rust               | State, Cue resolution, Preflight, OverlayState generation, save, API |
| Overlay               | Vanilla TS + CSS   | Render OverlayState                                                  |
| OBS Bridge            | Rust               | obs-websocket integration                                            |
| ExtensionRegistry     | Rust model         | plugin origin, capability, metadata management                       |
| ContributionValidator | Rust               | plugin contribution validation                                       |
| CLI                   | Rust               | validate, preflight, render fixture, inspect                         |

### 26.3 State Ownership

```text
Runtime:
  Source of truth for ProjectPackage
  Source of truth for ShowDefinition
  Source of truth for RunSession
  Source of truth for TypedEntity / DataTable
  Source of truth for PreviewSnapshot
  Source of truth for ProgramSnapshot
  Source of truth for Preflight results
  Source of truth for OBS connection status
  Source of truth for Overlay connection status
  Source of truth for ExtensionRegistry metadata
  Source of truth for Undo / Redo history
  Source of truth for Operation Log

Editor:
  selection
  viewport
  text input draft
  drag / resize transient state
  panel UI state

Overlay:
  latest received OverlayState snapshot
  knows nothing about ProjectPackage / ShowDefinition / RunSession / Cue / Slot / Plugin

Plugin, post-MVP:
  draft contribution submitted to Runtime
  no direct ownership of Runtime state
```

---

## 27. Runtime Design

### 27.1 Rust Crates

```text
cuecanvas-model:
  ProjectPackage / ShowDefinition / RunSession / Stage / OutputTarget / Template / Slot / Entity / DataTable / Cue / Snapshot / OverlayState / Origin / Extensions

cuecanvas-engine:
  CueEngine / TemplateResolver / SlotBindingResolver / DataResolver / OverlayStateBuilder

cuecanvas-preflight:
  Preflight rules / severity / blocking policy / fix actions / override hash

cuecanvas-project:
  .cuecanvas read/write / assets / migration / autosave / recovery / unknown extension metadata preservation

cuecanvas-runtime:
  HTTP server / WebSocket / state actor / command dispatcher / revision management

cuecanvas-obs:
  OBS WebSocket integration / setup wizard backend / source health check

cuecanvas-extension:
  ExtensionRegistry / PluginManifest / CapabilityPolicy / ContributionValidator

cuecanvas-render-test:
  Chromium golden render / fixture screenshot / overflow detection

cuecanvas-protocol:
  Runtime API types / Command / Event / Actor

cuecanvas-cli:
  validate / migrate / render-state / inspect-project / preflight
```

### 27.2 Runtime State

```text
- ProjectPackage
- activeShowDefinitionId
- activeRunSessionId
- PreviewSnapshot
- ProgramSnapshot
- retainedProgramSnapshot
- ExternalInputState
- ExtensionRegistry
- Project revision
- ShowDefinition revision
- Data revision
- Preview revision
- Program revision
- Runtime revision
- Preflight results
- Undo / Redo history
- Operation log
- Overlay connection status
- OBS connection status
- OBS source health
```

### 27.3 State Actor

All Runtime state updates go through a single state actor.

```text
Command
→ actor validation
→ capability check
→ idempotency check
→ expectedRevision check
→ state update
→ derived state rebuild
→ preflight update if needed
→ event broadcast
→ autosave schedule
→ operation log
→ ack
```

### 27.4 Command Categories

```text
ShowCommand:
  show.create
  show.updateMetadata
  show.updateStage

RunSessionCommand:
  run.create
  run.open
  run.close
  run.reset

OutputCommand:
  output.create
  output.update
  output.verify
  output.refresh

TemplateCommand:
  templateInstance.create
  templateInstance.update
  templateInstance.delete
  templateInstance.bindSlot

DataCommand:
  entity.create
  entity.update
  entity.delete
  dataTable.importCsv
  dataTable.pasteTsv
  dataTable.updateCell
  dataTable.updateSchema

CueCommand:
  cue.create
  cue.update
  cue.updateSlotOverride
  cue.updateEntityRef
  cue.updateTemplateState
  cue.reorder
  cue.delete

LiveCommand:
  live.enterRehearsal
  live.arm
  live.start
  live.previewCue
  live.refreshPreview
  live.next
  live.prev
  live.take
  live.clear
  live.blackout
  live.restoreProgram
  live.retakeCurrent
  live.end

PreflightCommand:
  preflight.runShow
  preflight.runCue
  preflight.runPreview
  preflight.overrideWarning

OBSCommand:
  obs.connect
  obs.disconnect
  obs.setupBrowserSource
  obs.verifySource
  obs.showTestPattern

ExtensionCommand, post-MVP:
  extension.install
  extension.uninstall
  extension.enable
  extension.disable
  extension.applyContribution
  extension.rejectContribution

ExternalInputCommand:
  externalInput.update
  externalInput.clear

ProjectCommand:
  project.open
  project.save
  project.saveAs
  project.recover
```

### 27.5 Event Categories

```text
project.changed
showDefinition.changed
runSession.changed
data.changed
preview.changed
program.changed
preflight.changed
overlay.connectionChanged
overlay.rendered
obs.connectionChanged
obs.healthChanged
extension.changed
contribution.received
contribution.applied
autosave.changed
runtime.error
operation.logged
```

---

## 28. Runtime API

### 28.1 HTTP

```http
GET  /health
GET  /editor
GET  /overlay/program
GET  /overlay/preview
GET  /assets/:assetId
GET  /fonts/:fontId

GET  /api/project
POST /api/project/open
POST /api/project/save
POST /api/project/save-as

GET  /api/show/:showDefinitionId
GET  /api/run/:runSessionId
POST /api/run/create

GET  /api/overlay/program/snapshot
GET  /api/overlay/preview/snapshot

GET  /api/preflight
POST /api/preflight/run

POST /api/data/import-csv
POST /api/data/paste-tsv

POST /api/obs/connect
POST /api/obs/setup-browser-source
POST /api/obs/verify
POST /api/obs/test-pattern

POST /api/external-input/:inputKey
POST /api/runtime/shutdown
```

Post-MVP extension API:

```http
GET  /api/extensions
POST /api/extensions/install
POST /api/extensions/:pluginId/enable
POST /api/extensions/:pluginId/disable
POST /api/extensions/:pluginId/uninstall
POST /api/extensions/contributions/apply
POST /api/extensions/contributions/reject
```

### 28.2 WebSocket

```http
WS /ws/editor
WS /ws/overlay/program
WS /ws/overlay/preview
```

### 28.3 Editor WebSocket

Editor → Runtime:

```text
command request
```

Runtime → Editor:

```text
project.changed
showDefinition.changed
runSession.changed
data.changed
preview.changed
program.changed
preflight.changed
obs.connectionChanged
obs.healthChanged
overlay.connectionChanged
overlay.rendered
extension.changed
contribution.received
contribution.applied
runtime.error
autosave.changed
operation.logged
```

---

## 29. Storage Format

### 29.1 .cuecanvas Package

```text
project.cuecanvas/
├── manifest.json
├── project.json
├── show-definitions/
│   └── show-main.json
├── run-sessions/
│   └── run-2026-05-04-main.json
├── extensions/
│   ├── registry.json
│   └── metadata/
├── assets/
│   ├── images/
│   └── videos/
├── fonts/
├── thumbnails/
├── renders/
│   ├── cue-thumbnails/
│   └── golden/
└── autosave/
    ├── latest.project.json
    ├── latest.show-definition.json
    ├── latest.run-session.json
    ├── latest.program-snapshot.json
    ├── retained.program-snapshot.json
    ├── command-log.jsonl
    └── recovery-info.json
```

MVP uses directory package format.

### 29.2 manifest.json

```json
{
  "format": "cuecanvas",
  "formatVersion": 5,
  "createdBy": "CueCanvas",
  "minimumRuntimeVersion": "0.1.0"
}
```

### 29.3 Extension Registry

```json
{
  "installedPlugins": [],
  "knownOrigins": [
    {
      "kind": "plugin",
      "pluginId": "com.example.event-pack",
      "pluginVersion": "1.0.0",
      "status": "missing"
    }
  ]
}
```

In the MVP, registry may be empty, but the system must be able to record origin for plugin-created artifacts.

### 29.4 Atomic Save

```text
1. Write to temporary file
2. flush / fsync
3. atomic rename to target json
4. update manifest
5. update extension registry
6. update latest ProgramSnapshot
7. update autosave
8. clean old autosaves
```

### 29.5 Unknown Extension Metadata Preservation

Save processing preserves unknown extension metadata.

```text
- Validate known schema
- For unknown extensions, validate only namespace / size / JSON validity
- Round-trip on save
- Do not delete during migration without explicit policy
```

---

## 30. Security

### 30.1 Runtime Server

```text
- Bind only to 127.0.0.1
- Editor API requires session token
- Overlay API requires read-only token
- Overlay token cannot operate Project / Show APIs
- External Input API is OFF by default
- External Input uses dedicated token
- Host header validation
- Origin validation
- CORS disabled by default
- payload size limit
- path traversal protection
- canonical path validation for assets / fonts
- inputKey character restrictions
- rate limit
```

### 30.2 Token Policy

```text
Editor session token:
  Generated per launch

Overlay token:
  Stable per installation or outputTarget
  Read-only
  Stored in Keychain

External input token:
  Per input group
  OFF by default
  Stored in Keychain

OBS password:
  Not stored in project.json
  Stored in Keychain

Plugin token, post-MVP:
  Per plugin installation
  Capability-scoped
  Revocable
```

### 30.3 Overlay URL and Safety

Overlay URL is stable but least-privilege.

```text
Overlay token allows:
  - Fetch Overlay HTML
  - Fetch Program snapshot
  - Connect Overlay WebSocket

Overlay token forbids:
  - Read Project
  - Edit Cue
  - Take
  - Clear / Blackout
  - Asset import
  - External input update
  - Plugin command
```

### 30.4 Plugin Security Policy

If plugins are implemented post-MVP, require:

```text
- capability declaration
- user approval
- plugin origin display
- network access policy
- file access policy
- timeout
- memory limit
- crash isolation
- signature / trust status, later
- disable / revoke path
- safe mode startup
```

### 30.5 Native Plugin Policy

Native plugins are not allowed by default.

Reasons:

```text
- Runtime crash risk
- notarization complexity
- ABI compatibility
- privilege escalation
- debugging difficulty
```

---

## 31. External Input

### 31.1 Basic Policy

External Input does not write directly to TemplateInstances or TextNodes.

It supplies values to Slot or Data context.

```http
POST /api/external-input/:inputKey
```

```json
{
  "value": "Taro Yamada"
}
```

### 31.2 Reflection Modes

```text
latched:
  External Input value is fixed into ProgramSnapshot at Take time.
  Default.

live:
  External Input updates immediately reflect in Program.
  For clock, score, comments, etc.

preview-only:
  Reflected in Preview, but not in Program until Take.
```

### 31.3 Relationship with Plugin Providers

External Input provider plugins are post-MVP.

```text
- live mode is a dangerous capability
- latched is default
- immediate Program reflection requires explicit opt-in
- Operation Log records updates
```

---

## 32. Undo / Redo

### 32.1 Basic Policy

Undo / Redo is limited to edit operations.

```text
Cmd+Z:
  Undo TemplateInstance edits, Data edits, Cue edits, Slot edits

Live operation:
  Take, Clear, Blackout, Restore, External Input update are separate from normal Undo
```

### 32.2 Target Operations

```text
- TemplateInstance create / update / delete
- TemplateInstance position / size update
- slot binding update
- entity create / update / delete
- dataTable cell update
- cue create / delete
- cue slot override update
- cue entityRef update
- cue reorder
- asset import
- plugin contribution apply, post-MVP
```

### 32.3 Operation Log

Live operations are recorded in Operation Log.

```text
12:01:04 Enter Live Armed
12:01:06 Start Live
12:01:10 Preview Cue 002
12:01:12 Take Cue 002
12:03:10 Blackout
12:03:12 Restore Program
```

Operation Log is not the Undo stack.

### 32.4 Operation Log Targets

```text
- Live state transition
- Preview Cue
- Take
- Clear
- Blackout
- Restore
- Preflight warning override
- OBS connection degraded
- Overlay reconnect
- Browser Source health warning
- Runtime recovery
- Plugin contribution applied, post-MVP
- Plugin disabled / enabled, post-MVP
```

---

## 33. Test Strategy

### 33.1 Engine Fixture Tests

```text
- typed entity resolution
- DataTable row resolution
- local slot binding resolution
- slotOverrides resolution
- TemplateInstance visibility resolution
- absolute cue resolution
- missing slot fallback
- missing asset warning
- PreviewSnapshot generation
- ProgramSnapshot immutability
- Clear / Blackout / Restore
- origin round-trip
- extensions metadata round-trip
```

### 33.2 Property-Based Tests

```text
- Direct jump to Cue produces deterministic result
- Cue order does not affect absolute Cue resolution
- Take does not re-resolve Cue
- PreviewSnapshot does not change after creation
- ProgramSnapshot does not change after creation
- OverlayState does not include invisible items
- disabled output item is not rendered
- TemplateInstance zIndex is stable
- unknown extensions are not lost on save
```

### 33.3 Preflight Tests

```text
- required local slot missing -> Error
- slot binding missing -> Error
- missing asset -> Error
- text overflow -> Warning
- OBS source size mismatch -> Error / Warning depending setting
- Program divergence -> Warning
- PreviewSnapshot stale -> Warning
- output disconnected -> Error before Live
- plugin-created artifact missing dependency -> Warning / Error
```

### 33.4 Renderer Tests

```text
- OverlayState fixture render
- Chromium screenshot comparison
- font fallback test
- CJK line breaking test
- image fit test
- text overflow detection
- transparent background test
```

### 33.5 Runtime / WebSocket Tests

```text
- overlay.hello returns full snapshot
- reconnect returns latest ProgramSnapshot
- overlay.rendered updates OverlayRendered
- stale sequence is ignored
- broadcast works for multiple overlay clients
- session mismatch is detected
- live.take idempotency works
- command actor appears in Operation Log
```

### 33.6 OBS E2E

```text
- OBS Setup Wizard can create Browser Source
- Browser Source URL is correct
- width / height matches Stage
- risky source refresh / shutdown settings are detected
- nested scene placement is verified
- Take changes the display
- Browser Source refresh restores Program
- scene switch preserves recovery
- source hide/show preserves recovery
```

### 33.7 Extension Readiness Tests

MVP has no plugin loader, but must test:

```text
- origin field round-trip
- extensions metadata round-trip
- command actor validation
- operation log actor recording
- unsupported extension metadata is preserved
- unknown plugin origin can be shown as warning
- artifact source chain can be shown in Diff / Inspector
```

---

## 34. MVP Scope

v5.2 defines the MVP as a Template-first / Run-first / plugin-ready-but-plugin-free minimum viable production system.

### 34.1 Technical Spike

Purpose:

```text
Confirm that ProgramSnapshot / OverlayState can be reliably output to OBS.
```

Include:

```text
- Rust Runtime skeleton
- hardcoded ShowDefinition fixture
- hardcoded RunSession fixture
- TemplateDefinition fixture
- TemplateInstance fixture
- 3 Cues
- CueEngine
- TemplateResolver
- SlotBindingResolver
- OverlayStateBuilder
- PreviewSnapshot generation
- ProgramSnapshot commit
- origin field
- extensions metadata field
- /overlay/program
- Overlay Renderer
- manual OBS Browser Source display
- Take CLI or simple HTTP command
- reconnect recovery
```

Exclude:

```text
- Board Editor
- SwiftUI Host
- save
- Undo / Redo
- CSV import UI
- OBS Scene Binding
- plugin loader
```

### 34.2 Operator Cockpit Prototype

Purpose:

```text
Validate the core CueCanvas experience: Preview / Program / Take / Restore.
```

Include:

```text
- Minimal Web Control Panel
- Cue List
- Preview selection
- PreviewSnapshot display
- Take
- Clear
- Blackout
- Restore
- Program monitor
- Overlay connection status
- OBS basic status
- textual Diff Preview
- Operation Log basic
- actor display basic
```

### 34.3 OBS Setup Wizard Slice

Purpose:

```text
Establish early Browser Source creation, configuration, and verification.
```

Include:

```text
- OBS connection
- Authentication
- Scene selection
- Browser Source create / update
- URL update
- width / height update
- custom CSS recommended setting
- source refresh / shutdown warning
- nested scene recommendation
- source verification
- Test Pattern
```

### 34.4 Alpha

Purpose:

```text
Allow users to build ShowDefinition using Templates, Data, and Cue Sheet.
```

Include:

```text
- Template Library basic
- TemplateInstance placement
- TemplateInstance Inspector
- local slot definitions
- People editor
- Sessions editor
- Sponsors editor
- CSV import
- TSV paste
- Column mapping
- Cue Sheet / Run Sheet
- Cue thumbnail generation basic
- Preflight initial
- origin / extension metadata preservation
```

### 34.5 Beta

Purpose:

```text
Reach quality suitable for testing in real events.
```

Include:

```text
- .cuecanvas package
- atomic save
- autosave / recovery
- latest ProgramSnapshot persistence
- Undo / Redo
- Preflight blocking policy
- macOS Host
- WKWebView integration
- Keychain
- Operation Log
- Rehearsal Mode
- Live Mode state machine
- Renderer confidence model tests
- Extension readiness tests
```

### 34.6 MVP

Purpose:

```text
Operate a small event in production while preserving schema compatibility for future plugins.
```

MVP success criteria:

```text
1. Runs as a macOS app
2. Runtime launches automatically
3. New Show can be created
4. Stage size can be configured
5. OBS Setup Wizard can create or update Browser Source
6. OBS Browser Source URL / size / refresh / shutdown settings can be verified
7. Lower Third / Session Title / Sponsor Bug Templates can be added
8. People / Sessions / Sponsors can be created
9. CSV import or TSV paste works
10. Cue Sheet can edit Cues
11. Cue can be Previewed
12. PreviewSnapshot is generated
13. Diff Preview is visible
14. Preflight detects major problems
15. Take turns PreviewSnapshot into ProgramSnapshot
16. Take changes OBS Program
17. Program restores after OBS Browser Source reload
18. Program does not implicitly change after Cue / Data / Template edits
19. Clear / Blackout / Restore works
20. Project can save and restore as .cuecanvas
21. origin / actor / extensions metadata can save and restore
22. unknown extension metadata is not lost
```

---

## 35. Roadmap

### Phase 0: Product Contract / Schema Contract

Artifacts:

```text
- product-contract.md
- live-safety.md
- show-definition-model.md
- run-session-model.md
- template-model.md
- cue-model.md
- overlay-contract.md
- preflight-policy.md
- extension-readiness.md
```

Invariants:

```text
- Program changes only through Take or explicit Live operation
- Take is PreviewSnapshot commit
- ProgramSnapshot is immutable
- PreviewSnapshot does not change after creation
- Cue direct jump resolves deterministically
- Overlay obtains latest ProgramSnapshot on reconnect
- Overlay URL does not usually require reconfiguration
- Editor is not the source of truth for ShowDefinition / RunSession
- External Input does not immediately affect Program by default
- Clear and Blackout are distinct
- Preflight Error may block Live Start / Take
- OBS Scene Binding does not change Program in MVP
- Plugin does not directly modify Program / Overlay / OBS
```

### Phase 1: Monorepo / Schema

```text
- Cargo workspace
- pnpm workspace
- model crate
- protocol crate
- JSON Schema generation
- TS type generation
- fixtures
- origin schema
- extensions metadata schema
- actor schema
```

### Phase 2: Engine + Fixture

```text
- ProjectPackage model
- ShowDefinition model
- RunSession model
- Stage / OutputTarget
- TemplateDefinition
- TemplateInstance
- local slot model
- typed entity model
- DataTable model
- CueSheet / Cue
- absolute cue resolution
- entityRefs resolution
- slotOverrides resolution
- templateStates application
- PreviewSnapshot generation
- ProgramSnapshot commit
- OverlayStateBuilder
- fixture tests
```

### Phase 3: Overlay Renderer / OBS Technical Spike / Setup Wizard Basic

```text
- Axum server
- /overlay/program
- Overlay Renderer
- WebSocket
- overlay.hello
- overlay.rendered
- full snapshot
- Text item rendering
- Image item rendering
- transparent background
- manual OBS Browser Source display
- OBS WebSocket connection
- basic Browser Source create / update
- URL / width / height verification
- reload recovery
```

### Phase 4: Operator Cockpit

```text
- Cue list
- Preview selection
- PreviewSnapshot display
- Take
- Clear
- Blackout
- Restore
- Program monitor
- Overlay connection status
- OBS health basic
- Diff Preview text
- Operation Log basic
- actor / origin display basic
```

### Phase 5: Data / Cue Sheet

```text
- People editor
- Sessions editor
- Sponsors editor
- CSV import
- TSV paste
- Column mapping
- Cue create / edit / reorder
- entity row selection
- slot override editor
- thumbnail generation
```

### Phase 6: Template Instance Editor

```text
- Template Library
- Stage view
- TemplateInstance placement
- position / size
- variant selection
- local slot binding
- safe area preview
- Inspector
```

### Phase 7: Preflight / Rehearsal

```text
- Preflight rules
- severity
- blocking policy
- fix actions
- warning override hash
- Preflight panel
- Rehearsal Mode
- Program divergence warning
- Preview stale warning
```

### Phase 8: Persistence / Extension Metadata

```text
- .cuecanvas package
- project.json read/write
- show-definitions/
- run-sessions/
- extension registry stub
- assets/images
- atomic save
- autosave
- latest ProgramSnapshot persistence
- recovery
- migration
- unknown extension metadata preservation
```

### Phase 9: macOS Host

```text
- SwiftUI app
- Runtime launcher
- Runtime monitor
- WKWebView
- Native menu
- File open / save / save-as
- Recent files
- Keychain
- Preferences
```

### Phase 10: OBS Setup Wizard Hardening

```text
- Authentication UX
- Scene selection
- Browser Source create / update
- URL update
- width / height update
- custom CSS recommendation
- source refresh / shutdown setting verification
- nested scene verification
- Test Pattern
- Health Check panel
```

### Phase 11: Undo / Redo / Live Safety

```text
- Edit command history
- Cue edit history
- Data edit history
- Operation log
- Live Mode restrictions
- destructive confirmations
- state machine enforcement
- idempotent live commands
```

### Phase 12: Renderer Confidence / E2E

```text
- Chromium golden render
- screenshot fixtures
- CJK text fixture
- font fallback fixture
- OBS E2E refresh
- OBS E2E hide/show
- OBS E2E scene switch
```

### Phase 13: Extension Readiness Hardening

```text
- PluginManifest schema draft
- Capability model draft
- ContributionValidator skeleton
- ExtensionRegistry UI read-only
- missing plugin origin warning
- plugin-created artifact badge support
- safe mode design
```

### Phase 14: Advanced Board Designer

```text
- TextNode placement
- ImageNode placement
- ShapeNode
- Layer Panel
- group
- component editing
- TemplateDefinition editor
- exposed properties
```

### Phase 15: External Input

```text
- ExternalInput model
- token
- API
- latched / live / preview-only
- SlotBindingResolver integration
- External input monitor
```

### Phase 16: Declarative Plugin Pack

```text
- manifest loader
- Template pack import
- Theme pack import
- Asset pack import
- CSV mapping preset import
- schema validation
- origin tracking
- enable / disable
```

### Phase 17: Data Importer / Cue Generator

```text
- External connector API
- typedEntityImport contribution
- cueGeneration contribution
- user apply / reject flow
- capability approval
- Operation Log integration
```

### Phase 18: Preflight Plugin

```text
- read-only rule execution
- PreflightResult contribution
- rule namespace validation
- fix action allowlist
- timeout
- safe mode
```

### Phase 19: OBS Scene Binding

```text
- Scene event subscription
- contextOnly binding
- preview binding
- applyMode policy
- debounce
- anti-loop guard
- Operation Log integration
- Preflight integration
- program / autoTakeCue remains explicit opt-in later feature
```

### Phase 20: Renderer Extension Research

```text
- component API design
- sandbox policy
- fixture tests
- OBS Verified Render requirement
- performance budget
- fallback rendering
```

### Phase 21: Template Library / Import / Sharing

```text
- built-in template expansion
- template export / import
- Show starter templates
- CSV mapping presets
```

### Phase 22: Windows Host Preparation

```text
- WebView2 Host policy
- Runtime launcher policy
- Credential Manager policy
- packaging policy
```

---

## 36. First Demo Specification

The first v5.2 demo prioritizes a vertical Show Run slice, not Board Editor.

### 36.1 Demo Show

```text
ShowDefinition: Main Event
Stage: 1920x1080
OutputTarget: OBS Program Overlay
RunSession: Main Rehearsal

Templates:
  - Lower Third Basic
  - Session Title
  - Sponsor Bug

Typed Entities:
  People:
    - Cat / CTO / cat.png
    - Dog / CFO / dog.png

  Sessions:
    - Opening
    - Technical Session
    - Closing

  Sponsors:
    - ACME / acme.png / gold

CueSheet:
  001 Opening Title
  002 CTO Cat Lower Third
  003 Sponsor Bug
  004 CFO Dog Lower Third
  005 Closing Title
```

### 36.2 Demo Flow

```text
1. Start Runtime
2. Open Minimal Control Panel
3. Set /overlay/program in OBS Browser Source, or create it with Wizard
4. Pass OBS Health Check
5. Preview Cue 001
6. PreviewSnapshot is generated
7. Check Diff Preview
8. Take
9. PreviewSnapshot becomes ProgramSnapshot
10. OBS Program becomes Opening
11. Preview Cue 002
12. Confirm Cat / CTO / cat.png in Diff Preview
13. Take
14. OBS Program becomes Lower Third
15. Refresh Browser Source
16. Latest ProgramSnapshot is restored
17. Overlay ACKs programRevision
18. Blackout
19. Restore Program
20. Save / reopen preserves origin / extensions metadata
```

### 36.3 Demo Success Criteria

```text
- Cue 001 can be Previewed
- PreviewSnapshot revision is issued
- Diff is computed between ProgramSnapshot and PreviewSnapshot
- Take succeeds with specified previewRevision
- ProgramRevision increments
- Overlay ACKs programRevision
- Same ProgramSnapshot restores after OBS Browser Source refresh
- Blackout retains retainedProgramSnapshot
- Restore restores retainedProgramSnapshot
- Cue / Data / Template edits do not change Program
- Command actor appears in Operation Log
- Artifact origin appears in Inspector
- Unknown extension metadata is not lost after save / restore
```

Do not proceed to Board Designer or advanced Template editing until this demo succeeds.

---

## 37. Success Metrics

### 37.1 Technical Success

```text
- Direct Cue jump produces deterministic OverlayState
- PreviewSnapshot generation p95 <= 100ms
- Take command accept to overlay ACK p95 <= 100ms
- Latest ProgramSnapshot restores after Overlay reconnect
- Browser Source refresh recovery within 1 second
- Program does not implicitly change after Cue / Data / Template edits
- Full snapshot delivery avoids state loss
- OBS Setup Wizard creates Browser Source correctly
- live.take idempotency is guaranteed
- origin / actor / extensions metadata round-trip is preserved
```

### 37.2 UX Success

```text
- Speaker data can be edited in Cue Sheet without confusion
- Cues can be created from CSV import / TSV paste
- Preview / Program difference is always clear
- Diff is visible before Take
- Preflight warnings are practical
- Panic controls are clear
- Operation works without creating many OBS sources
- First-time user can complete Cue 001 -> Preview -> Take within 10 minutes
- Source is clear even for plugin-created artifacts
```

### 37.3 Extension Success, Post-MVP

```text
- Template pack can be imported safely
- Plugin-created Template origin is visible
- Data importer does not directly change Program
- Cue generator does not bypass Preview / Take workflow
- Preflight plugin does not bypass Runtime policy
- Project remains valid after plugin disable
- Safe mode can open Project with plugins disabled
```

---

## 38. Risks and Countermeasures

### 38.1 Being Pulled Toward Board Editor

Risk:

```text
Development spends too much time on freeform Board editing,
while Live Run / Cue Sheet / Preflight are delayed.
```

Countermeasures:

```text
- Exclude Board Editor from MVP
- Build TemplateInstance Editor first
- Move Advanced Board Designer to Phase 14 or later
- Do not start freeform editing until the first demo succeeds
```

### 38.2 Mixing ShowDefinition and RunSession

Risk:

```text
Prepared state and live execution state become mixed,
making ProgramSnapshot and Operation Log ambiguous.
```

Countermeasures:

```text
- Separate ShowDefinition / RunSession at model level
- Place ProgramSnapshot under RunSession
- Treat Rehearsal and Live as RunSession
- Store ShowDefinitionRevision in RunSession
```

### 38.3 Re-Resolving Cue at Take Time

Risk:

```text
Edits after Preview cause the displayed Preview to differ from actual Program.
```

Countermeasures:

```text
- Take is PreviewSnapshot commit
- live.take requires previewRevision
- Show stale preview warning
- Provide Refresh Preview action
```

### 38.4 Plugin Breaking Safety Boundaries

Risk:

```text
Plugin directly modifies Program / Overlay / OBS and breaks live safety.
```

Countermeasures:

```text
- Forbid plugin direct write
- Use contribution model
- Require Runtime validation
- Require Capability model
- Require Operation Log
- Explicit opt-in for dangerous capabilities
```

### 38.5 Implementing Plugin System Too Early

Risk:

```text
Building a plugin framework before MVP delays the core product.
```

Countermeasures:

```text
- MVP is plugin-free
- Add only plugin-ready schema
- Loader / marketplace / sandbox are post-MVP
- Limit Phase 13 to readiness hardening
```

### 38.6 Template Slot Coupled to Data Structure

Risk:

```text
External data names such as speaker.name enter TemplateDefinition,
reducing template reuse.
```

Countermeasures:

```text
- TemplateDefinition uses local slot contract
- TemplateInstance / Cue owns data binding
- Use generic slots such as primaryText / secondaryText / portrait
```

### 38.7 DataTable Becoming a Spreadsheet Product

Risk:

```text
Development drifts toward general spreadsheet features,
weakening Cue creation experience.
```

Countermeasures:

```text
- UI presents People / Sessions / Sponsors
- DataTable remains internal storage
- Initial schema is limited
- General DataTable editor is later
```

### 38.8 OBS Scene Binding Breaking Safety

Risk:

```text
OBS scene changes unexpectedly modify Program.
```

Countermeasures:

```text
- Exclude from MVP
- Start post-MVP with contextOnly / preview
- Make program / autoTake explicit opt-in
- Require Runtime validation and LiveCommand
- Always log in Operation Log
```

### 38.9 Overtrusting Renderer Match

Risk:

```text
Editor Preview differs from OBS output.
```

Countermeasures:

```text
- Formalize Renderer Confidence Model
- Restrict CSS
- Address font metrics
- Chromium screenshot tests
- OBS E2E
- Describe Preview as high-confidence, not exact guarantee
```

### 38.10 Unknown Extension Metadata Loss

Risk:

```text
Plugin metadata is lost across versions or save cycles.
```

Countermeasures:

```text
- Round-trip unknown extensions
- Validate namespace only
- Do not delete during migration
- Apply size limits
```

---

## 39. Documentation Split

```text
docs/product-plan.md
  Purpose, target users, non-goals, success metrics

docs/product-contract.md
  Invariants, Take semantics, ProgramSnapshot, Live safety

docs/extension-readiness.md
  Plugin-ready policy, origin, extensions, actor, capability

docs/plugin-model.md
  PluginManifest, contribution, lifecycle, security, sandbox policy

docs/show-definition-model.md
  ProjectPackage / ShowDefinition / Stage / OutputTarget

docs/run-session-model.md
  RunSession / PreviewSnapshot / ProgramSnapshot / Operation Log

docs/template-model.md
  TemplateDefinition / TemplateInstance / local slots / variants / bindings

docs/data-model.md
  TypedEntity / DataTable / CSV import / validation

docs/cue-model.md
  Cue, entityRefs, slotOverrides, templateStates, absolute cue

docs/overlay-state.md
  OverlayState schema, Renderer contract, sequence, session identity

docs/renderer-confidence.md
  Editor Preview, Chromium Golden Render, OBS Verified Render

docs/runtime-api.md
  HTTP / WebSocket / Command / Event / Actor

docs/security.md
  token, localhost, OBS secrets, External Input, Plugin security

docs/obs-setup.md
  OBS Setup Wizard, Browser Source, health check

docs/obs-scene-binding.md
  post-MVP Scene Binding policy

docs/cue-sheet.md
  Cue Sheet, Run Sheet, diff preview, thumbnail

docs/preflight.md
  Preflight rules, severity, blocking policy, fix actions

docs/live-state-machine.md
  Setup / Rehearsal / LiveArmed / LiveRunning / Recovery

docs/roadmap.md
  Phase, MVP, Demo, Release plan
```

---

## 40. Final Summary

CueCanvas v5.2 preserves the v5.1 direction of a Show Graphics System, while adding safe future extensibility within explicit boundaries.

The most important v5.2 position is:

```text
CueCanvas is not a Canvas app.
CueCanvas is a Show Graphics System.
CueCanvas is not a Plugin platform.
CueCanvas is a plugin-ready, live-safe system.
```

Core values in v5.2:

```text
- Choose Templates
- Feed People / Sessions / Sponsors into graphics
- Build Cue Sheet / Run Sheet
- Confirm with PreviewSnapshot
- Understand changes with Diff
- Prevent mistakes with Preflight
- Promote PreviewSnapshot to ProgramSnapshot with Take
- Restore from ProgramSnapshot after OBS reload or failure
- Keep future extensions under Runtime validation and capability control
```

Final v5.2 direction:

```text
Run-first.
Template-first.
Data-first.
Snapshot-safe.
OBS-safe.
Plugin-ready.
Plugin-free MVP.
```

The first thing to build is not a freeform Board Editor or a plugin framework.

The first thing to build is:

```text
A minimal Live Run system that can create a ShowDefinition,
load People / Sessions / Sponsors,
Preview a Cue,
Take a PreviewSnapshot,
safely output to OBS,
recover from ProgramSnapshot when problems occur,
and track future plugin-created artifacts.
```

Only after that core is proven should development proceed to Advanced Template Editor, External Input, Declarative Plugin Pack, Data Importer, Preflight Plugin, OBS Scene Binding, Renderer Extension, and Windows Host.
