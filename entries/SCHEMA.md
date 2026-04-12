# Module Markdown Schema

> **Purpose:** Defines the standard format for all training module files. Every module — bootcamp, P4P class, or future program — follows this schema so the hub can parse, display, and monitor it for product update impacts.

---

## Frontmatter (YAML)

Every module file starts with YAML frontmatter between `---` fences. This is the machine-readable layer that the hub and automation skills consume.

```yaml
---
id: M04                              # Unique identifier
title: "API Design & Architecture"   # Display name
program: bootcamp                    # Program this belongs to (bootcamp, p4p, etc.)
owner: Marcus Kidan                  # Who maintains this module
change_sensitivity: medium           # low | medium | high
status: current                      # current | needs_review | update_in_progress | outdated
last_updated: 2026-04-10            # Date content was last revised
last_audited: 2026-04-10            # Date someone last checked against current product

postman_features:                    # Product features this module depends on
  - API Builder
  - Mock Servers
  - OpenAPI spec support

update_triggers:                     # What kinds of product changes should flag this module
  - "New spec format support"
  - "Design tooling UI changes"
  - "Mock server feature updates"
---
```

### Field Reference

| Field | Required | Type | Purpose |
|-------|----------|------|---------|
| `id` | Yes | string | Unique module ID (M01, P4P-101, etc.) |
| `title` | Yes | string | Human-readable name |
| `program` | Yes | string | Which program this belongs to |
| `owner` | Yes | string | Content owner |
| `change_sensitivity` | Yes | enum | How often this needs product-driven updates |
| `status` | Yes | enum | Current content freshness |
| `last_updated` | Yes | date | When content was last revised |
| `last_audited` | Yes | date | When last checked against current product |
| `postman_features` | Yes | list | Specific features referenced — the linkage for update monitoring |
| `update_triggers` | Yes | list | Plain-language descriptions of what product changes affect this module |

---

## Body Sections (Markdown)

After the frontmatter, the body uses standard markdown headings. These are the sections the hub renders.

### Required Sections

**`## Overview`** — What this module covers and why it matters. 2–4 sentences.

**`## Learning Objectives`** — What learners will understand or be able to do after this module. Bulleted list, 2–4 items.

**`## Key Elements`** — The core teaching content. This is the substance of the module — concepts, frameworks, examples, metaphors. Use subheadings as needed.

**`## Activities`** — Hands-on exercises, discussion prompts, or group work. Each activity should have a clear setup and expected outcome.

**`## Postman Features in This Module`** — How specific Postman product capabilities are used or demonstrated. This section exists so the hub can show *what* about each feature is taught, not just *that* it's referenced.

### Optional Sections

**`## Facilitator Notes`** — Guidance for whoever is delivering this module. Timing, common questions, what to emphasize.

**`## Pain Points Addressed`** — Which real-world problems this module surfaces and contextualizes.

---

## File Naming Convention

### Module Files (in subdirectories)

```
entries/
  bootcamp/
    M01-welcome.md
    M02-internet-apis.md
    ...
  p4p/                    # Future
    P4P-101-api-basics.md
    P4P-201-advanced.md
```

Pattern: `{ID}-{slug}.md` where the slug is a lowercase, hyphenated short name.

### Entry Files / Notebooks (in entries/ root)

```
entries/
  meeting-notes.md       ← Any name works
  standups.md
  project-alpha.md
  SCHEMA.md              ← Excluded (system file)
  SYSTEM.md              ← Excluded (system file)
```

Entry files are auto-discovered by the HTML viewer. The filename becomes the display name (hyphens and underscores become spaces, title-cased). Name files however makes sense — the system doesn't depend on any naming convention.

**Critical rule:** Every entry file must start with a date heading as its first line:

```markdown
# MM-DD-YYYY
```

This is what the parser uses to split the file into individual entries. Without a date on the first line, the file's content will not be displayed.

### Directory-Based Notebooks (one file per entry)

As an alternative to single-file notebooks, you can organize entries as individual `.md` files inside a subdirectory of `entries/`. The directory name becomes the notebook name.

```
entries/
  apple-notes-2025/          ← Directory notebook: "Apple Notes 2025"
    2025-06-15 Sprint Retro.md
    2025-06-22 Design Review.md
    2025-07-01 Planning.md
  imported-journal/          ← Directory notebook: "Imported Journal"
    01-05-2025 Morning thoughts.md
    01-12-2025 Project ideas.md
```

**Date in filename:** The date is extracted from the filename prefix. Supported formats:

| Format | Example filename |
|--------|-----------------|
| `YYYY-MM-DD Title.md` | `2025-06-15 Sprint Retro.md` (ISO, Obsidian Importer default) |
| `YYYYMMDD Title.md` | `20250615 Sprint Retro.md` |
| `MM-DD-YYYY Title.md` | `06-15-2025 Sprint Retro.md` (matches existing schema) |

The separator between the date and the title can be a space, hyphen, or underscore. Files without a recognizable date prefix are skipped.

**File content:** Each file's content is the entry body. If the file starts with a `# MM-DD-YYYY` heading, it is stripped to avoid duplicate date display. If the filename contains a title and the body doesn't start with a heading, the title is prepended as an `## h2`.

**Tagging:** Tags work identically in directory notebooks — select text, apply a tag, and the tag span is written back to the individual entry file.

**When to use which format:**

- **Single-file notebooks** are best for content you write natively in the app, or for small focused notebooks where one file is easier to manage.
- **Directory notebooks** are ideal for imported content (Apple Notes via Obsidian Importer, exported journals, etc.) where each note is already a separate file. They also work well when you want individual notes to be independently version-controlled or edited outside the app.

---

## How the Hub Uses This

1. **Discovery:** On load, the hub scans the `entries/` folder for all `.md` files (single-file notebooks, excluding `SCHEMA.md` and `SYSTEM.md`) and all subdirectories containing `.md` files (directory notebooks). No manual registration is needed.
2. **Parsing (modules):** For module files, the hub reads frontmatter YAML to populate module cards, status indicators, and the feature dependency map.
3. **Parsing (entries):** For entry files, the hub splits content at date headings (`# MM-DD-YYYY`) and renders each as a timestamped entry.
4. **Monitoring:** The `postman_features` list in module files is compared against incoming product updates. A match flags the module for review.
5. **Display:** Markdown body sections are rendered in the detail view. All entries across all files are merged into the Library timeline, sorted by date.
6. **Audit trail:** `last_updated` and `last_audited` dates in module frontmatter drive the "freshness" indicators on the dashboard.
